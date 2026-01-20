mod external;
mod internal;

pub use external::get_external_ip;
pub use internal::get_internal_ip;

use std::net::IpAddr;

use crate::config::IpSource;
use anyhow::Result;

pub async fn get_ip(source: &IpSource) -> Result<IpAddr> {
    match source {
        IpSource::External => get_external_ip().await,
        IpSource::Internal => get_internal_ip(),
    }
}
