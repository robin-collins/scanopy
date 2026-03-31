use std::net::IpAddr;
use std::time::Duration;

use anyhow::Error;
use tokio::time::timeout;

use super::NetworkScan;

impl NetworkScan {
    pub(super) async fn get_hostname_for_ip(&self, ip: IpAddr) -> Result<Option<String>, Error> {
        match timeout(Duration::from_millis(2000), async {
            tokio::task::spawn_blocking(move || dns_lookup::lookup_addr(&ip)).await?
        })
        .await
        {
            Ok(Ok(hostname)) => Ok(Some(hostname)),
            _ => Ok(None),
        }
    }
}
