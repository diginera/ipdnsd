use std::time::Duration;

use anyhow::{anyhow, Context, Result};
use async_trait::async_trait;
use reqwest::Client;
use serde::{Deserialize, Serialize};

use super::provider::{Credentials, DnsProvider, DnsRecord};

const GODADDY_API_BASE: &str = "https://api.godaddy.com/v1";
const REQUEST_TIMEOUT: Duration = Duration::from_secs(30);

pub struct GoDaddyProvider {
    client: Client,
    credentials: Credentials,
}

#[derive(Debug, Serialize, Deserialize)]
struct GoDaddyRecord {
    data: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    ttl: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename = "type")]
    record_type: Option<String>,
}

impl GoDaddyProvider {
    pub fn new(credentials: Credentials) -> Self {
        let client = Client::builder()
            .timeout(REQUEST_TIMEOUT)
            .build()
            .expect("Failed to create HTTP client");

        Self { client, credentials }
    }

    fn auth_header(&self) -> String {
        format!(
            "sso-key {}:{}",
            self.credentials.api_key, self.credentials.api_secret
        )
    }
}

#[async_trait]
impl DnsProvider for GoDaddyProvider {
    async fn get_record(
        &self,
        domain: &str,
        record_type: &str,
        name: &str,
    ) -> Result<DnsRecord> {
        let url = format!(
            "{}/domains/{}/records/{}/{}",
            GODADDY_API_BASE, domain, record_type, name
        );

        let response = self
            .client
            .get(&url)
            .header("Authorization", self.auth_header())
            .send()
            .await
            .context("Failed to send request to GoDaddy API")?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            return Err(anyhow!(
                "GoDaddy API error ({}): {}",
                status,
                body
            ));
        }

        let records: Vec<GoDaddyRecord> = response
            .json()
            .await
            .context("Failed to parse GoDaddy API response")?;

        let record = records
            .into_iter()
            .next()
            .ok_or_else(|| anyhow!("No record found for {}.{}", name, domain))?;

        Ok(DnsRecord {
            name: name.to_string(),
            record_type: record_type.to_string(),
            data: record.data,
            ttl: record.ttl.unwrap_or(600),
        })
    }

    async fn update_record(&self, domain: &str, record: &DnsRecord) -> Result<()> {
        let url = format!(
            "{}/domains/{}/records/{}/{}",
            GODADDY_API_BASE, domain, record.record_type, record.name
        );

        let payload = vec![GoDaddyRecord {
            data: record.data.clone(),
            name: None,
            ttl: Some(record.ttl),
            record_type: None,
        }];

        let response = self
            .client
            .put(&url)
            .header("Authorization", self.auth_header())
            .header("Content-Type", "application/json")
            .json(&payload)
            .send()
            .await
            .context("Failed to send update request to GoDaddy API")?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            return Err(anyhow!(
                "GoDaddy API error ({}): {}",
                status,
                body
            ));
        }

        Ok(())
    }

    fn provider_name(&self) -> &'static str {
        "godaddy"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_auth_header() {
        let provider = GoDaddyProvider::new(Credentials {
            api_key: "test_key".to_string(),
            api_secret: "test_secret".to_string(),
        });

        assert_eq!(provider.auth_header(), "sso-key test_key:test_secret");
    }

    #[test]
    fn test_godaddy_record_serialization() {
        let record = GoDaddyRecord {
            data: "192.168.1.1".to_string(),
            name: None,
            ttl: Some(600),
            record_type: None,
        };

        let json = serde_json::to_string(&record).unwrap();
        assert!(json.contains("192.168.1.1"));
        assert!(json.contains("600"));
        assert!(!json.contains("name")); // None fields should be skipped
    }
}
