mod cloudflare;
mod godaddy;
mod provider;

pub use cloudflare::CloudflareProvider;
pub use godaddy::GoDaddyProvider;
pub use provider::{Credentials, DnsProvider, DnsRecord};

use anyhow::{anyhow, Result};
use std::sync::Arc;

pub fn create_provider(name: &str, credentials: Credentials) -> Result<Arc<dyn DnsProvider>> {
    match name.to_lowercase().as_str() {
        "cloudflare" => Ok(Arc::new(CloudflareProvider::new(credentials))),
        "godaddy" => Ok(Arc::new(GoDaddyProvider::new(credentials))),
        _ => Err(anyhow!("Unknown DNS provider: {}", name)),
    }
}
