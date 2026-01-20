use std::fmt;
use std::path::PathBuf;

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Settings {
    pub daemon: DaemonConfig,
    pub dns_entries: Vec<DnsEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DaemonConfig {
    #[serde(default = "default_interval")]
    pub interval_seconds: u64,
    #[serde(default = "default_log_level")]
    pub log_level: String,
}

fn default_interval() -> u64 {
    300 // 5 minutes
}

fn default_log_level() -> String {
    "info".to_string()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DnsEntry {
    pub provider: String,
    pub domain: String,
    pub record_name: String,
    pub record_type: String,
    pub ip_source: IpSource,
    #[serde(default)]
    pub ttl: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum IpSource {
    External,
    Internal,
}

impl fmt::Display for IpSource {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            IpSource::External => write!(f, "external"),
            IpSource::Internal => write!(f, "internal"),
        }
    }
}

impl Settings {
    pub fn load() -> Result<Self> {
        let config_path = Self::config_path();

        let content = std::fs::read_to_string(&config_path)
            .with_context(|| format!("Failed to read config file: {}", config_path.display()))?;

        let settings: Settings = toml::from_str(&content)
            .with_context(|| format!("Failed to parse config file: {}", config_path.display()))?;

        Ok(settings)
    }

    pub fn config_path() -> PathBuf {
        Self::config_dir().join("config.toml")
    }

    pub fn config_dir() -> PathBuf {
        #[cfg(unix)]
        {
            PathBuf::from("/etc/ipdnsd")
        }
        #[cfg(windows)]
        {
            PathBuf::from(r"C:\ProgramData\ipdnsd")
        }
    }
}

impl Default for DaemonConfig {
    fn default() -> Self {
        Self {
            interval_seconds: default_interval(),
            log_level: default_log_level(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_config() {
        let toml_str = r#"
[daemon]
interval_seconds = 300
log_level = "info"

[[dns_entries]]
provider = "godaddy"
domain = "example.com"
record_name = "@"
record_type = "A"
ip_source = "external"

[[dns_entries]]
provider = "godaddy"
domain = "example.com"
record_name = "internal"
record_type = "A"
ip_source = "internal"
ttl = 600
"#;

        let settings: Settings = toml::from_str(toml_str).unwrap();
        assert_eq!(settings.daemon.interval_seconds, 300);
        assert_eq!(settings.daemon.log_level, "info");
        assert_eq!(settings.dns_entries.len(), 2);
        assert_eq!(settings.dns_entries[0].provider, "godaddy");
        assert_eq!(settings.dns_entries[0].ip_source, IpSource::External);
        assert_eq!(settings.dns_entries[1].ip_source, IpSource::Internal);
        assert_eq!(settings.dns_entries[1].ttl, Some(600));
    }
}
