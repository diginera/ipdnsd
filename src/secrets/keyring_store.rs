use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

use anyhow::{anyhow, Context, Result};
use serde::{Deserialize, Serialize};

use crate::dns::Credentials;

#[derive(Debug, Default, Serialize, Deserialize)]
struct CredentialsFile {
    #[serde(default)]
    providers: HashMap<String, ProviderCredentials>,
}

#[derive(Debug, Serialize, Deserialize)]
struct ProviderCredentials {
    api_key: String,
    api_secret: String,
}

fn credentials_path() -> PathBuf {
    #[cfg(unix)]
    {
        PathBuf::from("/etc/ipdnsd/credentials.toml")
    }
    #[cfg(windows)]
    {
        PathBuf::from(r"C:\ProgramData\ipdnsd\credentials.toml")
    }
}

fn load_credentials_file() -> Result<CredentialsFile> {
    let path = credentials_path();
    if !path.exists() {
        return Ok(CredentialsFile::default());
    }

    let content = fs::read_to_string(&path)
        .with_context(|| format!("Failed to read credentials file: {}", path.display()))?;

    toml::from_str(&content)
        .with_context(|| format!("Failed to parse credentials file: {}", path.display()))
}

fn save_credentials_file(creds: &CredentialsFile) -> Result<()> {
    let path = credentials_path();

    // Ensure parent directory exists
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("Failed to create directory: {}", parent.display()))?;
    }

    let content = toml::to_string_pretty(creds)
        .context("Failed to serialize credentials")?;

    fs::write(&path, &content)
        .with_context(|| format!("Failed to write credentials file: {}", path.display()))?;

    // Set restrictive permissions on Unix
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let perms = fs::Permissions::from_mode(0o600);
        fs::set_permissions(&path, perms)
            .with_context(|| format!("Failed to set permissions on: {}", path.display()))?;
    }

    Ok(())
}

pub fn store_credentials(provider: &str, api_key: &str, api_secret: &str) -> Result<()> {
    let mut creds_file = load_credentials_file()?;

    creds_file.providers.insert(
        provider.to_string(),
        ProviderCredentials {
            api_key: api_key.to_string(),
            api_secret: api_secret.to_string(),
        },
    );

    save_credentials_file(&creds_file)?;

    Ok(())
}

pub fn get_credentials(provider: &str) -> Result<Credentials> {
    let creds_file = load_credentials_file()?;

    let provider_creds = creds_file
        .providers
        .get(provider)
        .ok_or_else(|| {
            anyhow!(
                "Credentials not found for provider: {}. Use 'ipdnsd set-key {}' to store credentials.",
                provider,
                provider
            )
        })?;

    Ok(Credentials {
        api_key: provider_creds.api_key.clone(),
        api_secret: provider_creds.api_secret.clone(),
    })
}

pub fn delete_credentials(provider: &str) -> Result<()> {
    let mut creds_file = load_credentials_file()?;

    if creds_file.providers.remove(provider).is_none() {
        return Err(anyhow!("No credentials found for provider: {}", provider));
    }

    save_credentials_file(&creds_file)?;

    Ok(())
}
