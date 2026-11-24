//! Tiny authoritative DNS server for the local TLD. The OS resolver is
//! pointed at it for `*.test` only (see `locdev-platform`), so it never sees
//! or answers any other traffic.

pub mod wire;

use std::collections::VecDeque;
use std::sync::{Arc, Mutex};

use tokio::net::UdpSocket;
use tracing::{debug, warn};

/// How many recently asked names to remember for the routing self-check.
const RECENT: usize = 64;

pub struct DnsServer {
    tld: String,
    recent: Mutex<VecDeque<String>>,
}

impl DnsServer {
    pub fn new(tld: &str) -> Arc<Self> {
        Arc::new(Self {
            tld: tld.to_ascii_lowercase(),
            recent: Mutex::new(VecDeque::with_capacity(RECENT)),
        })
    }

    /// Whether this server was asked about `name` recently. Lets the daemon
    /// prove that the OS routes the TLD here (and not to another tool).
    pub fn saw(&self, name: &str) -> bool {
        let name = name.to_ascii_lowercase();
        self.recent.lock().unwrap().iter().any(|n| *n == name)
    }

    fn remember(&self, name: String) {
        let mut recent = self.recent.lock().unwrap();
        if recent.len() == RECENT {
            recent.pop_front();
        }
        recent.push_back(name);
    }

    /// Answers queries until the task is dropped/aborted.
    pub async fn serve(self: Arc<Self>, socket: UdpSocket) {
        let mut buf = [0u8; 1500];
        loop {
            let (len, peer) = match socket.recv_from(&mut buf).await {
                Ok(received) => received,
                Err(error) => {
                    // On Windows an ICMP "port unreachable" from a previous
                    // send surfaces here; it is harmless.
                    debug!(%error, "dns recv failed");
                    continue;
                }
            };

            let Some((response, name)) = wire::respond(&buf[..len], &self.tld) else {
                continue;
            };
            if let Some(name) = name {
                self.remember(name);
            }
            if let Err(error) = socket.send_to(&response, peer).await {
                warn!(%error, %peer, "dns send failed");
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn answers_over_udp() {
        let socket = UdpSocket::bind("127.0.0.1:0").await.unwrap();
        let addr = socket.local_addr().unwrap();
        let server = DnsServer::new("test");
        tokio::spawn(server.clone().serve(socket));

        let client = UdpSocket::bind("127.0.0.1:0").await.unwrap();
        client
            .send_to(&wire::query(7, "Probe-123.test", 1, true), addr)
            .await
            .unwrap();
        let mut buf = [0u8; 512];
        let (len, _) = client.recv_from(&mut buf).await.unwrap();

        assert_eq!(&buf[..2], &7u16.to_be_bytes());
        assert!(buf[..len].windows(4).any(|w| w == [127, 0, 0, 1]));
        assert!(server.saw("probe-123.test"));
        assert!(!server.saw("other.test"));
    }
}
