use std::net::IpAddr;
use std::time::Duration;

use anyhow::{anyhow, Result};
use reqwest::Client;

const EXTERNAL_IP_SERVICES: &[&str] = &[
    "https://api.ipify.org",
    "https://ifconfig.me/ip",
    "https://icanhazip.com",
    "https://ipinfo.io/ip",
    "https://checkip.amazonaws.com",
];

const REQUEST_TIMEOUT: Duration = Duration::from_secs(10);

pub async fn get_external_ip() -> Result<IpAddr> {
    let client = Client::builder()
        .timeout(REQUEST_TIMEOUT)
        .build()?;

    let mut last_error = None;

    for service in EXTERNAL_IP_SERVICES {
        match fetch_ip(&client, service).await {
            Ok(ip) => return Ok(ip),
            Err(e) => {
                tracing::debug!("Failed to get IP from {}: {}", service, e);
                last_error = Some(e);
            }
        }
    }

    Err(last_error.unwrap_or_else(|| anyhow!("No IP services available")))
}

async fn fetch_ip(client: &Client, url: &str) -> Result<IpAddr> {
    let response = client
        .get(url)
        .send()
        .await?
        .error_for_status()?
        .text()
        .await?;

    let ip_str = response.trim();
    let ip: IpAddr = ip_str.parse()?;

    Ok(ip)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_fetch_ip_format() {
        // Test that we can parse typical IP responses
        let test_cases = vec![
            "192.168.1.1",
            "192.168.1.1\n",
            "  10.0.0.1  ",
            "2001:db8::1",
        ];

        for case in test_cases {
            let ip: Result<IpAddr, _> = case.trim().parse();
            assert!(ip.is_ok(), "Failed to parse: {}", case);
        }
    }
}
