use std::time::Duration;

use anyhow::{anyhow, Context, Result};
use async_trait::async_trait;
use reqwest::Client;
use serde::{Deserialize, Serialize};

use super::provider::{Credentials, DnsProvider, DnsRecord};

const CLOUDFLARE_API_BASE: &str = "https://api.cloudflare.com/client/v4";
const REQUEST_TIMEOUT: Duration = Duration::from_secs(30);

pub struct CloudflareProvider {
    client: Client,
    credentials: Credentials,
}

#[derive(Debug, Deserialize)]
struct CloudflareResponse<T> {
    success: bool,
    result: T,
    errors: Vec<CloudflareError>,
}

#[derive(Debug, Deserialize)]
struct CloudflareError {
    message: String,
}

#[derive(Debug, Deserialize)]
struct Zone {
    id: String,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct CloudflareRecord {
    id: String,
    name: String,
    #[serde(rename = "type")]
    record_type: String,
    content: String,
    ttl: u32,
}

#[derive(Debug, Serialize)]
struct CloudflareRecordUpdate {
    #[serde(rename = "type")]
    record_type: String,
    name: String,
    content: String,
    ttl: u32,
}

impl CloudflareProvider {
    pub fn new(credentials: Credentials) -> Self {
        let client = Client::builder()
            .timeout(REQUEST_TIMEOUT)
            .build()
            .expect("Failed to create HTTP client");

        Self { client, credentials }
    }

    fn auth_header(&self) -> String {
        format!("Bearer {}", self.credentials.api_key)
    }

    async fn resolve_zone_id(&self, domain: &str) -> Result<String> {
        let url = format!("{}/zones?name={}", CLOUDFLARE_API_BASE, domain);

        let response = self
            .client
            .get(&url)
            .header("Authorization", self.auth_header())
            .send()
            .await
            .context("Failed to send zone lookup request to Cloudflare API")?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            return Err(anyhow!("Cloudflare API error ({}): {}", status, body));
        }

        let resp: CloudflareResponse<Vec<Zone>> = response
            .json()
            .await
            .context("Failed to parse Cloudflare zone response")?;

        if !resp.success {
            let msgs: Vec<_> = resp.errors.iter().map(|e| e.message.as_str()).collect();
            return Err(anyhow!("Cloudflare API error: {}", msgs.join(", ")));
        }

        resp.result
            .into_iter()
            .next()
            .map(|z| z.id)
            .ok_or_else(|| anyhow!("No zone found for domain: {}", domain))
    }
}

/// Build the fully qualified domain name from a record name and domain.
///
/// - `"@"` + `"example.com"` → `"example.com"`
/// - `"sub"` + `"example.com"` → `"sub.example.com"`
fn build_fqdn(name: &str, domain: &str) -> String {
    if name == "@" {
        domain.to_string()
    } else {
        format!("{}.{}", name, domain)
    }
}

#[async_trait]
impl DnsProvider for CloudflareProvider {
    async fn get_record(
        &self,
        domain: &str,
        record_type: &str,
        name: &str,
    ) -> Result<DnsRecord> {
        let zone_id = self.resolve_zone_id(domain).await?;
        let fqdn = build_fqdn(name, domain);

        let url = format!(
            "{}/zones/{}/dns_records?type={}&name={}",
            CLOUDFLARE_API_BASE, zone_id, record_type, fqdn
        );

        let response = self
            .client
            .get(&url)
            .header("Authorization", self.auth_header())
            .send()
            .await
            .context("Failed to send request to Cloudflare API")?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            return Err(anyhow!("Cloudflare API error ({}): {}", status, body));
        }

        let resp: CloudflareResponse<Vec<CloudflareRecord>> = response
            .json()
            .await
            .context("Failed to parse Cloudflare API response")?;

        if !resp.success {
            let msgs: Vec<_> = resp.errors.iter().map(|e| e.message.as_str()).collect();
            return Err(anyhow!("Cloudflare API error: {}", msgs.join(", ")));
        }

        let record = resp
            .result
            .into_iter()
            .next()
            .ok_or_else(|| anyhow!("No {} record found for {}", record_type, fqdn))?;

        Ok(DnsRecord {
            name: name.to_string(),
            record_type: record.record_type,
            data: record.content,
            ttl: record.ttl,
        })
    }

    async fn update_record(&self, domain: &str, record: &DnsRecord) -> Result<()> {
        let zone_id = self.resolve_zone_id(domain).await?;
        let fqdn = build_fqdn(&record.name, domain);

        // First, find the record ID
        let list_url = format!(
            "{}/zones/{}/dns_records?type={}&name={}",
            CLOUDFLARE_API_BASE, zone_id, record.record_type, fqdn
        );

        let response = self
            .client
            .get(&list_url)
            .header("Authorization", self.auth_header())
            .send()
            .await
            .context("Failed to send request to Cloudflare API")?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            return Err(anyhow!("Cloudflare API error ({}): {}", status, body));
        }

        let resp: CloudflareResponse<Vec<CloudflareRecord>> = response
            .json()
            .await
            .context("Failed to parse Cloudflare API response")?;

        if !resp.success {
            let msgs: Vec<_> = resp.errors.iter().map(|e| e.message.as_str()).collect();
            return Err(anyhow!("Cloudflare API error: {}", msgs.join(", ")));
        }

        let existing = resp
            .result
            .into_iter()
            .next()
            .ok_or_else(|| {
                anyhow!(
                    "No existing {} record found for {} to update",
                    record.record_type,
                    fqdn
                )
            })?;

        // Now update the record by ID
        let update_url = format!(
            "{}/zones/{}/dns_records/{}",
            CLOUDFLARE_API_BASE, zone_id, existing.id
        );

        let payload = CloudflareRecordUpdate {
            record_type: record.record_type.clone(),
            name: fqdn,
            content: record.data.clone(),
            ttl: record.ttl,
        };

        let response = self
            .client
            .put(&update_url)
            .header("Authorization", self.auth_header())
            .json(&payload)
            .send()
            .await
            .context("Failed to send update request to Cloudflare API")?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            return Err(anyhow!("Cloudflare API error ({}): {}", status, body));
        }

        Ok(())
    }

    fn provider_name(&self) -> &'static str {
        "cloudflare"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_build_fqdn_root() {
        assert_eq!(build_fqdn("@", "example.com"), "example.com");
    }

    #[test]
    fn test_build_fqdn_subdomain() {
        assert_eq!(build_fqdn("sub", "example.com"), "sub.example.com");
    }

    #[test]
    fn test_build_fqdn_deep_subdomain() {
        assert_eq!(build_fqdn("a.b", "example.com"), "a.b.example.com");
    }

    #[test]
    fn test_auth_header() {
        let provider = CloudflareProvider::new(Credentials {
            api_key: "test_token".to_string(),
            api_secret: String::new(),
        });

        assert_eq!(provider.auth_header(), "Bearer test_token");
    }

    #[test]
    fn test_record_update_serialization() {
        let update = CloudflareRecordUpdate {
            record_type: "A".to_string(),
            name: "example.com".to_string(),
            content: "192.168.1.1".to_string(),
            ttl: 600,
        };

        let json = serde_json::to_value(&update).unwrap();
        assert_eq!(json["type"], "A");
        assert_eq!(json["name"], "example.com");
        assert_eq!(json["content"], "192.168.1.1");
        assert_eq!(json["ttl"], 600);
    }
}
