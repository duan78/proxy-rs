use std::net::Ipv4Addr;

use byteorder::{BigEndian, WriteBytesExt};
use tokio::io::AsyncReadExt;

use crate::proxy::Proxy;

#[derive(Debug, Clone)]
pub struct Socks4Negotiator {
    pub name: String,
    pub check_anon_lvl: bool,
    pub use_full_path: bool,
}

impl Socks4Negotiator {
    pub async fn negotiate(&self, proxy: &mut Proxy) -> bool {
        let bip = proxy.host.parse::<Ipv4Addr>();
        if bip.is_err() {
            return false;
        }

        let mut buf = Vec::with_capacity(9); // Pre-allocate buffer
        let _ = buf.write_u8(4); // SOCKS version
        let _ = buf.write_u8(1); // Command
        let _ = buf.write_u16::<BigEndian>(proxy.port);
        if let Ok(ip) = bip {
            buf.extend_from_slice(&ip.octets());
        } else {
            return false;
        }
        let _ = buf.write_u8(0); // Reserved

        let packet = buf;

        proxy.send(packet.as_slice()).await;

        if let Some(data) = proxy.recv(8).await {
            let mut data = data.as_slice();

            let version = data.read_u8().await;
            if version.is_err() || version.unwrap() != 0 {
                proxy.log(
                    "Invalid response version",
                    None,
                    Some("invalid_response_version".to_string()),
                );
                return false;
            }

            let resp = data.read_u8().await;
            if resp.is_err() || resp.unwrap() != 90 {
                proxy.log(
                    "Request rejected or Failed",
                    None,
                    Some("request_failed".to_string()),
                );
                return false;
            }

            proxy.log("Request is granted", None, None);
            return true;
        }
        false
    }
}

impl Default for Socks4Negotiator {
    fn default() -> Self {
        Self {
            name: "SOCKS4".to_string(),
            check_anon_lvl: false,
            use_full_path: false,
        }
    }
}
