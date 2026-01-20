use std::net::IpAddr;

use anyhow::Result;
use async_trait::async_trait;

#[derive(Debug, Clone)]
pub struct Credentials {
    pub api_key: String,
    pub api_secret: String,
}

#[derive(Debug, Clone)]
pub struct DnsRecord {
    pub name: String,
    pub record_type: String,
    pub data: String,
    pub ttl: u32,
}

impl DnsRecord {
    pub fn new(name: &str, record_type: &str, ip: IpAddr, ttl: u32) -> Self {
        Self {
            name: name.to_string(),
            record_type: record_type.to_string(),
            data: ip.to_string(),
            ttl,
        }
    }
}

#[async_trait]
pub trait DnsProvider: Send + Sync {
    /// Get a DNS record for the specified domain
    async fn get_record(
        &self,
        domain: &str,
        record_type: &str,
        name: &str,
    ) -> Result<DnsRecord>;

    /// Update a DNS record
    async fn update_record(&self, domain: &str, record: &DnsRecord) -> Result<()>;

    /// Get the provider name
    fn provider_name(&self) -> &'static str;
}
