use anyhow::Result;
use clap::{Parser, Subcommand};
use tracing::info;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};

use ipdnsd::{config::Settings, daemon, dns::create_provider, ip, secrets};

#[derive(Parser)]
#[command(name = "ipdnsd")]
#[command(about = "IP to DNS Updater - monitors IP addresses and updates DNS records")]
#[command(version)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Run the daemon to monitor IP changes and update DNS
    Daemon,

    /// Check current IPs and DNS records
    Check,

    /// Store API credentials for a DNS provider
    SetKey {
        /// DNS provider name (e.g., godaddy)
        provider: String,
    },

    /// Delete stored API credentials for a DNS provider
    DeleteKey {
        /// DNS provider name (e.g., godaddy)
        provider: String,
    },

    /// Show configuration file location and contents
    Config,

    /// Install as a system service
    Install,

    /// Uninstall the system service
    Uninstall,
}

fn init_logging(log_level: &str) {
    let filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new(log_level));

    tracing_subscriber::registry()
        .with(filter)
        .with(tracing_subscriber::fmt::layer())
        .init();
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    // Load config for commands that need it
    let settings = Settings::load().ok();

    // Initialize logging with config level or default
    let log_level = settings
        .as_ref()
        .map(|s| s.daemon.log_level.as_str())
        .unwrap_or("info");
    init_logging(log_level);

    match cli.command {
        Commands::Daemon => {
            let settings = settings.ok_or_else(|| {
                anyhow::anyhow!("Configuration file not found. Run 'ipdnsd config' to see the expected location.")
            })?;
            info!("Starting ipdnsd daemon");
            daemon::run(settings).await?;
        }

        Commands::Check => {
            check_status().await?;
        }

        Commands::SetKey { provider } => {
            use std::io::{self, Write};

            print!("API Key: ");
            io::stdout().flush()?;
            let mut key = String::new();
            io::stdin().read_line(&mut key)?;
            let key = key.trim();

            let secret = rpassword::prompt_password("API Secret: ")?;

            secrets::store_credentials(&provider, key, &secret)?;
            println!("Credentials stored for provider: {}", provider);
        }

        Commands::DeleteKey { provider } => {
            secrets::delete_credentials(&provider)?;
            println!("Credentials deleted for provider: {}", provider);
        }

        Commands::Config => {
            show_config(&settings)?;
        }

        Commands::Install => {
            daemon::install_service()?;
            println!("Service installed successfully");
        }

        Commands::Uninstall => {
            daemon::uninstall_service()?;
            println!("Service uninstalled successfully");
        }
    }

    Ok(())
}

async fn check_status() -> Result<()> {
    println!("Checking IP addresses...\n");

    // Check external IP
    match ip::get_external_ip().await {
        Ok(ip) => println!("External IP: {}", ip),
        Err(e) => println!("External IP: Error - {}", e),
    }

    // Check internal IP
    match ip::get_internal_ip() {
        Ok(ip) => println!("Internal IP: {}", ip),
        Err(e) => println!("Internal IP: Error - {}", e),
    }

    // If we have a config, check DNS records
    if let Ok(settings) = Settings::load() {
        println!("\nChecking DNS records...\n");

        for entry in &settings.dns_entries {
            let creds = match secrets::get_credentials(&entry.provider) {
                Ok(c) => c,
                Err(e) => {
                    println!(
                        "{}.{} ({}): Error getting credentials - {}",
                        entry.record_name, entry.domain, entry.provider, e
                    );
                    continue;
                }
            };

            let provider = create_provider(&entry.provider, creds)?;

            match provider
                .get_record(&entry.domain, &entry.record_type, &entry.record_name)
                .await
            {
                Ok(record) => {
                    println!(
                        "{}.{} ({}): {} -> {}",
                        entry.record_name,
                        entry.domain,
                        entry.record_type,
                        entry.provider,
                        record.data
                    );
                }
                Err(e) => {
                    println!(
                        "{}.{} ({}): Error - {}",
                        entry.record_name, entry.domain, entry.provider, e
                    );
                }
            }
        }
    } else {
        println!("\nNo configuration file found. DNS records not checked.");
    }

    Ok(())
}

fn show_config(settings: &Option<Settings>) -> Result<()> {
    let config_path = Settings::config_path();

    println!("Configuration file location: {}\n", config_path.display());

    match settings {
        Some(s) => {
            println!("Current configuration:\n");
            println!("{}", toml::to_string_pretty(s)?);
        }
        None => {
            println!("Configuration file not found.");
            println!("\nCreate a configuration file at the location above.");
            println!("Example configuration:\n");
            println!(
                r#"[daemon]
interval_seconds = 300
log_level = "info"

[[dns_entries]]
provider = "godaddy"
domain = "example.com"
record_name = "@"
record_type = "A"
ip_source = "external"
"#
            );
        }
    }

    Ok(())
}
