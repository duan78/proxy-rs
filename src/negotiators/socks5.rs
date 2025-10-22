use std::net::Ipv4Addr;

use byteorder::{BigEndian, WriteBytesExt};

use crate::proxy::Proxy;

#[derive(Debug, Clone)]
pub struct Socks5Negotiator {
    pub name: String,
    pub check_anon_lvl: bool,
    pub use_full_path: bool,
}

impl Socks5Negotiator {
    pub async fn negotiate(&self, proxy: &mut Proxy) -> bool {
        let packet = [5, 1, 0];
        proxy.send(&packet).await;
        if let Some(data) = proxy.recv(2).await {
            if data[0] != 0x05 {
                proxy.log("Invalid version", None, Some("invalid_version".to_string()));
                return false;
            }
            if data[1] == 0xff {
                proxy.log(
                    "Failed (auth is required)",
                    None,
                    Some("auth_is_required".to_string()),
                );
                return false;
            }
            if data[1] != 0x00 {
                proxy.log(
                    "Failed (invalid data)",
                    None,
                    Some("invalid_data".to_string()),
                );
                return false;
            }

            let bip = proxy.host.parse::<Ipv4Addr>();
            if bip.is_err() {
                return false;
            }

            let mut buf = Vec::with_capacity(10); // Pre-allocate buffer
            let _ = buf.write_u8(5); // SOCKS version
            let _ = buf.write_u8(1); // Number of auth methods
            let _ = buf.write_u8(0); // No authentication
            let _ = buf.write_u8(1); // IPv4
            if let Ok(ip) = bip {
            buf.extend_from_slice(&ip.octets());
        } else {
            return false;
        }
            let _ = buf.write_u16::<BigEndian>(proxy.port);

            let packet = buf;

            proxy.send(packet.as_slice()).await;
            if let Some(data) = proxy.recv(10).await {
                if data[0] != 0x05 || data[1] != 0x00 {
                    proxy.log(
                        "Failed (invalid data)",
                        None,
                        Some("invalid_data".to_string()),
                    );
                    return false;
                }
                proxy.log("Request is granted", None, None);
                return true;
            }
        }
        false
    }
}

impl Default for Socks5Negotiator {
    fn default() -> Self {
        Self {
            name: "SOCKS5".to_string(),
            check_anon_lvl: false,
            use_full_path: false,
        }
    }
}
