use std::collections::HashMap;
use std::net::IpAddr;
use std::sync::Arc;
use std::time::Duration;

use anyhow::Result;
use tokio::signal;
use tokio::sync::watch;
use tracing::{error, info, warn};

use crate::config::{DnsEntry, Settings};
use crate::dns::{create_provider, DnsProvider, DnsRecord};
use crate::ip;
use crate::secrets;

pub async fn run(settings: Settings) -> Result<()> {
    let interval = Duration::from_secs(settings.daemon.interval_seconds);

    // Create shutdown channel
    let (shutdown_tx, mut shutdown_rx) = watch::channel(false);

    // Spawn shutdown signal handler
    tokio::spawn(async move {
        if let Err(e) = wait_for_shutdown().await {
            error!("Error waiting for shutdown signal: {}", e);
        }
        let _ = shutdown_tx.send(true);
    });

    // Cache for last known IPs
    let mut ip_cache: HashMap<String, IpAddr> = HashMap::new();

    // Pre-load providers
    let mut providers: HashMap<String, Arc<dyn DnsProvider>> = HashMap::new();
    for entry in &settings.dns_entries {
        if !providers.contains_key(&entry.provider) {
            match secrets::get_credentials(&entry.provider) {
                Ok(creds) => match create_provider(&entry.provider, creds) {
                    Ok(provider) => {
                        providers.insert(entry.provider.clone(), provider);
                    }
                    Err(e) => {
                        error!("Failed to create provider {}: {}", entry.provider, e);
                    }
                },
                Err(e) => {
                    error!(
                        "Failed to get credentials for {}: {}",
                        entry.provider, e
                    );
                }
            }
        }
    }

    info!(
        "Daemon started. Monitoring {} DNS entries with {} second interval",
        settings.dns_entries.len(),
        settings.daemon.interval_seconds
    );

    // Initial check
    check_and_update(&settings.dns_entries, &providers, &mut ip_cache).await;

    // Main loop
    loop {
        tokio::select! {
            _ = tokio::time::sleep(interval) => {
                check_and_update(&settings.dns_entries, &providers, &mut ip_cache).await;
            }
            _ = shutdown_rx.changed() => {
                if *shutdown_rx.borrow() {
                    info!("Shutdown signal received, stopping daemon");
                    break;
                }
            }
        }
    }

    Ok(())
}

async fn check_and_update(
    entries: &[DnsEntry],
    providers: &HashMap<String, Arc<dyn DnsProvider>>,
    ip_cache: &mut HashMap<String, IpAddr>,
) {
    for entry in entries {
        let cache_key = format!("{}:{}:{}", entry.ip_source, entry.domain, entry.record_name);

        // Get current IP
        let current_ip = match ip::get_ip(&entry.ip_source).await {
            Ok(ip) => ip,
            Err(e) => {
                warn!(
                    "Failed to get {:?} IP for {}.{}: {}",
                    entry.ip_source, entry.record_name, entry.domain, e
                );
                continue;
            }
        };

        // Check if IP changed
        let ip_changed = match ip_cache.get(&cache_key) {
            Some(cached_ip) => *cached_ip != current_ip,
            None => true, // First run, need to check DNS
        };

        if !ip_changed {
            continue;
        }

        // Update cache
        ip_cache.insert(cache_key.clone(), current_ip);

        // Get provider
        let provider = match providers.get(&entry.provider) {
            Some(p) => p,
            None => {
                warn!("Provider {} not available", entry.provider);
                continue;
            }
        };

        // Check current DNS record
        let dns_record = match provider
            .get_record(&entry.domain, &entry.record_type, &entry.record_name)
            .await
        {
            Ok(record) => record,
            Err(e) => {
                warn!(
                    "Failed to get DNS record for {}.{}: {}",
                    entry.record_name, entry.domain, e
                );
                // Still try to update
                DnsRecord::new(
                    &entry.record_name,
                    &entry.record_type,
                    current_ip,
                    entry.ttl.unwrap_or(600),
                )
            }
        };

        // Check if DNS needs update
        let current_ip_str = current_ip.to_string();
        if dns_record.data == current_ip_str {
            info!(
                "DNS record {}.{} already set to {}",
                entry.record_name, entry.domain, current_ip
            );
            continue;
        }

        // Update DNS
        let new_record = DnsRecord::new(
            &entry.record_name,
            &entry.record_type,
            current_ip,
            entry.ttl.unwrap_or(dns_record.ttl),
        );

        info!(
            "Updating {}.{} from {} to {}",
            entry.record_name, entry.domain, dns_record.data, current_ip
        );

        match provider.update_record(&entry.domain, &new_record).await {
            Ok(()) => {
                info!(
                    "Successfully updated {}.{} to {}",
                    entry.record_name, entry.domain, current_ip
                );
            }
            Err(e) => {
                error!(
                    "Failed to update {}.{}: {}",
                    entry.record_name, entry.domain, e
                );
            }
        }
    }
}

async fn wait_for_shutdown() -> Result<()> {
    #[cfg(unix)]
    {
        use tokio::signal::unix::{signal, SignalKind};
        let mut sigterm = signal(SignalKind::terminate())?;
        let mut sigint = signal(SignalKind::interrupt())?;

        tokio::select! {
            _ = sigterm.recv() => {
                info!("Received SIGTERM");
            }
            _ = sigint.recv() => {
                info!("Received SIGINT");
            }
        }
    }

    #[cfg(windows)]
    {
        signal::ctrl_c().await?;
        info!("Received Ctrl+C");
    }

    Ok(())
}

#[cfg(unix)]
pub fn install_service() -> Result<()> {
    use std::process::Command;

    let exe_path = std::env::current_exe()?;
    let service_file = format!(
        r#"[Unit]
Description=IP to DNS Updater
After=network-online.target
Wants=network-online.target

[Service]
Type=simple
ExecStart={}  daemon
Restart=always
RestartSec=10

[Install]
WantedBy=multi-user.target
"#,
        exe_path.display()
    );

    let service_path = "/etc/systemd/system/ipdnsd.service";

    // Write service file
    std::fs::write(service_path, service_file)?;

    // Reload systemd and enable service
    Command::new("systemctl").args(["daemon-reload"]).status()?;
    Command::new("systemctl")
        .args(["enable", "ipdnsd"])
        .status()?;
    Command::new("systemctl")
        .args(["start", "ipdnsd"])
        .status()?;

    info!("Service installed and started");
    Ok(())
}

#[cfg(unix)]
pub fn uninstall_service() -> Result<()> {
    use std::process::Command;

    Command::new("systemctl")
        .args(["stop", "ipdnsd"])
        .status()?;
    Command::new("systemctl")
        .args(["disable", "ipdnsd"])
        .status()?;

    let service_path = "/etc/systemd/system/ipdnsd.service";
    if std::path::Path::new(service_path).exists() {
        std::fs::remove_file(service_path)?;
    }

    Command::new("systemctl").args(["daemon-reload"]).status()?;

    info!("Service uninstalled");
    Ok(())
}

#[cfg(windows)]
pub fn install_service() -> Result<()> {
    use anyhow::anyhow;

    // On Windows, we'll use sc.exe for simplicity
    // For a production service, consider using windows-service crate properly
    let exe_path = std::env::current_exe()?;

    let status = std::process::Command::new("sc")
        .args([
            "create",
            "ipdnsd",
            "binPath=",
            &format!("\"{}\" daemon", exe_path.display()),
            "start=",
            "auto",
            "DisplayName=",
            "IP to DNS Updater",
        ])
        .status()?;

    if !status.success() {
        return Err(anyhow!("Failed to create Windows service"));
    }

    let status = std::process::Command::new("sc")
        .args(["start", "ipdnsd"])
        .status()?;

    if !status.success() {
        warn!("Service created but failed to start. You may need to start it manually.");
    }

    Ok(())
}

#[cfg(windows)]
pub fn uninstall_service() -> Result<()> {
    use anyhow::anyhow;

    // Stop the service first (ignore errors if not running)
    let _ = std::process::Command::new("sc")
        .args(["stop", "ipdnsd"])
        .status();

    let status = std::process::Command::new("sc")
        .args(["delete", "ipdnsd"])
        .status()?;

    if !status.success() {
        return Err(anyhow!("Failed to delete Windows service"));
    }

    Ok(())
}
