//! A capture-only SMTP server (RFC 5321) on loopback. It accepts every
//! message and never relays: nothing it receives leaves this machine.
//!
//! `AUTH PLAIN`/`LOGIN` are accepted with any password; LocDev sets the user
//! to the site's name, which is how a message is tied to its project.
//! STARTTLS isn't offered: it's loopback, and clients only upgrade when asked.

use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;

use locdev_core::mail::MailEnvelope;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::{TcpListener, TcpStream};
use tracing::{debug, warn};

/// Largest message accepted (announced with `SIZE`).
pub const MAX_SIZE: usize = 25 * 1024 * 1024;
const MAX_RECIPIENTS: usize = 200;
const IDLE: Duration = Duration::from_secs(120);

/// Receives every accepted message.
pub trait Sink: Send + Sync + 'static {
    fn deliver(&self, envelope: MailEnvelope, raw: Vec<u8>);
}

pub async fn serve(listener: TcpListener, sink: Arc<dyn Sink>) {
    loop {
        match listener.accept().await {
            Ok((stream, peer)) => {
                let sink = sink.clone();
                tokio::spawn(async move {
                    if let Err(error) = session(stream, peer, sink).await {
                        debug!(%error, %peer, "smtp session ended");
                    }
                });
            }
            Err(error) => warn!(%error, "smtp accept failed"),
        }
    }
}

fn now_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| d.as_millis() as u64)
}

fn decode_base64(input: &str) -> Option<Vec<u8>> {
    const TABLE: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = Vec::new();
    let mut buf = 0u32;
    let mut bits = 0;
    for c in input.trim().bytes().filter(|c| *c != b'=') {
        let v = TABLE.iter().position(|t| *t == c)? as u32;
        buf = (buf << 6) | v;
        bits += 6;
        if bits >= 8 {
            bits -= 8;
            out.push((buf >> bits) as u8);
        }
    }
    Some(out)
}

/// The user in an `AUTH PLAIN` response (`\0user\0pass` or `authz\0user\0pass`).
fn plain_user(response: &str) -> Option<String> {
    let bytes = decode_base64(response)?;
    let parts: Vec<&[u8]> = bytes.split(|b| *b == 0).collect();
    let user = parts.get(1)?;
    String::from_utf8(user.to_vec()).ok()
}

/// `<addr>` from `FROM:<addr> SIZE=…`.
fn path(arg: &str) -> String {
    let arg = arg.trim();
    match (arg.find('<'), arg.find('>')) {
        (Some(a), Some(b)) if b > a => arg[a + 1..b].to_string(),
        _ => arg.split_whitespace().next().unwrap_or("").to_string(),
    }
}

async fn session(stream: TcpStream, peer: SocketAddr, sink: Arc<dyn Sink>) -> std::io::Result<()> {
    let (read, mut write) = stream.into_split();
    let mut reader = BufReader::new(read);
    let mut line = String::new();
    let mut envelope = MailEnvelope {
        peer: peer.to_string(),
        protocol: "SMTP".into(),
        ..Default::default()
    };
    let mut have_from = false;

    macro_rules! reply {
        ($($arg:tt)*) => {{
            write.write_all(format!($($arg)*).as_bytes()).await?;
        }};
    }

    reply!("220 locdev ESMTP capture-only: nothing is delivered\r\n");
    loop {
        line.clear();
        let n = match tokio::time::timeout(IDLE, reader.read_line(&mut line)).await {
            Ok(r) => r?,
            Err(_) => {
                reply!("421 idle too long\r\n");
                return Ok(());
            }
        };
        if n == 0 {
            return Ok(());
        }
        if line.len() > 4096 {
            reply!("500 line too long\r\n");
            continue;
        }
        let input = line.trim_end_matches(['\r', '\n']);
        let (verb, arg) = input.split_once(' ').unwrap_or((input, ""));
        match verb.to_ascii_uppercase().as_str() {
            "EHLO" => {
                envelope.helo = Some(arg.trim().to_string());
                envelope.protocol = "ESMTP".into();
                reply!(
                    "250-locdev\r\n250-SIZE {MAX_SIZE}\r\n250-8BITMIME\r\n250-SMTPUTF8\r\n250-PIPELINING\r\n250 AUTH PLAIN LOGIN\r\n"
                );
            }
            "HELO" => {
                envelope.helo = Some(arg.trim().to_string());
                reply!("250 locdev\r\n");
            }
            "AUTH" => {
                let mut words = arg.split_whitespace();
                let mechanism = words.next().unwrap_or("").to_ascii_uppercase();
                let user = match mechanism.as_str() {
                    "PLAIN" => {
                        let response = match words.next() {
                            Some(r) => r.to_string(),
                            None => {
                                reply!("334 \r\n");
                                let mut l = String::new();
                                reader.read_line(&mut l).await?;
                                l.trim().to_string()
                            }
                        };
                        plain_user(&response)
                    }
                    "LOGIN" => {
                        let user = match words.next() {
                            Some(u) => u.to_string(),
                            None => {
                                reply!("334 VXNlcm5hbWU6\r\n");
                                let mut l = String::new();
                                reader.read_line(&mut l).await?;
                                l.trim().to_string()
                            }
                        };
                        reply!("334 UGFzc3dvcmQ6\r\n");
                        let mut pass = String::new();
                        reader.read_line(&mut pass).await?;
                        decode_base64(&user).and_then(|u| String::from_utf8(u).ok())
                    }
                    _ => {
                        reply!("504 unrecognized authentication type\r\n");
                        continue;
                    }
                };
                match user {
                    Some(user) => {
                        envelope.auth_user = Some(user);
                        if !envelope.protocol.ends_with("A") {
                            envelope.protocol.push('A');
                        }
                        reply!("235 authenticated (any password is accepted here)\r\n");
                    }
                    None => reply!("501 malformed authentication response\r\n"),
                }
            }
            "MAIL" => {
                let Some(rest) = arg
                    .get(..5)
                    .filter(|p| p.eq_ignore_ascii_case("FROM:"))
                    .map(|_| &arg[5..])
                else {
                    reply!("501 syntax: MAIL FROM:<address>\r\n");
                    continue;
                };
                envelope.from = path(rest);
                envelope.to.clear();
                have_from = true;
                reply!("250 ok\r\n");
            }
            "RCPT" => {
                let Some(rest) = arg
                    .get(..3)
                    .filter(|p| p.eq_ignore_ascii_case("TO:"))
                    .map(|_| &arg[3..])
                else {
                    reply!("501 syntax: RCPT TO:<address>\r\n");
                    continue;
                };
                if !have_from {
                    reply!("503 MAIL first\r\n");
                } else if envelope.to.len() >= MAX_RECIPIENTS {
                    reply!("452 too many recipients\r\n");
                } else {
                    envelope.to.push(path(rest));
                    reply!("250 ok\r\n");
                }
            }
            "DATA" => {
                if envelope.to.is_empty() {
                    reply!("503 RCPT first\r\n");
                    continue;
                }
                reply!("354 end with <CRLF>.<CRLF>\r\n");
                let mut raw = Vec::new();
                let mut too_big = false;
                loop {
                    let mut buf = Vec::new();
                    let n = tokio::time::timeout(IDLE, reader.read_until(b'\n', &mut buf))
                        .await
                        .map_err(|_| std::io::Error::other("data timed out"))??;
                    if n == 0 {
                        return Ok(());
                    }
                    if buf == b".\r\n" || buf == b".\n" {
                        break;
                    }
                    // Dot-stuffing (RFC 5321 §4.5.2).
                    let body = if buf.starts_with(b"..") {
                        &buf[1..]
                    } else {
                        &buf[..]
                    };
                    if raw.len() + body.len() > MAX_SIZE {
                        too_big = true;
                    } else {
                        raw.extend_from_slice(body);
                    }
                }
                if too_big {
                    reply!("552 message larger than {MAX_SIZE} bytes\r\n");
                } else {
                    let mut accepted = envelope.clone();
                    accepted.received_at = now_ms();
                    sink.deliver(accepted, raw);
                    reply!("250 captured by LocDev\r\n");
                }
                envelope.to.clear();
                have_from = false;
            }
            "RSET" => {
                envelope.to.clear();
                have_from = false;
                reply!("250 ok\r\n");
            }
            "NOOP" => reply!("250 ok\r\n"),
            "VRFY" => reply!("252 cannot verify, but will accept\r\n"),
            "STARTTLS" => reply!("502 TLS isn't offered on this loopback server\r\n"),
            "QUIT" => {
                reply!("221 bye\r\n");
                return Ok(());
            }
            _ => reply!("502 command not implemented\r\n"),
        }
    }
}
