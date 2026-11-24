//! A minimal SMTP client for LocDev's own server only (`locdev sendmail`,
//! test messages). It refuses anything but loopback.

use std::net::SocketAddr;

use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::TcpStream;

async fn expect(
    reader: &mut BufReader<tokio::net::tcp::OwnedReadHalf>,
    code: &str,
) -> Result<(), String> {
    loop {
        let mut line = String::new();
        if reader
            .read_line(&mut line)
            .await
            .map_err(|e| e.to_string())?
            == 0
        {
            return Err("the server closed the connection".into());
        }
        if !line.starts_with(code) {
            return Err(format!("unexpected reply: {}", line.trim()));
        }
        // `250-…` continues, `250 …` ends.
        if line.as_bytes().get(3) != Some(&b'-') {
            return Ok(());
        }
    }
}

/// Sends `raw` to the capture server at `addr` as `site` (the AUTH user).
pub async fn send(
    addr: SocketAddr,
    site: Option<&str>,
    from: &str,
    to: &[String],
    raw: &[u8],
) -> Result<(), String> {
    if !addr.ip().is_loopback() {
        return Err("LocDev only sends to its own loopback capture server".into());
    }
    if to.is_empty() {
        return Err("no recipient".into());
    }
    let stream = TcpStream::connect(addr)
        .await
        .map_err(|e| format!("LocDev's mail server isn't reachable on {addr}: {e}"))?;
    let (read, mut write) = stream.into_split();
    let mut reader = BufReader::new(read);
    expect(&mut reader, "220").await?;
    write
        .write_all(b"EHLO locdev\r\n")
        .await
        .map_err(|e| e.to_string())?;
    expect(&mut reader, "250").await?;
    if let Some(site) = site {
        let mut plain = vec![0u8];
        plain.extend_from_slice(site.as_bytes());
        plain.extend_from_slice(b"\0locdev");
        write
            .write_all(format!("AUTH PLAIN {}\r\n", crate::parse::base64(&plain)).as_bytes())
            .await
            .map_err(|e| e.to_string())?;
        expect(&mut reader, "235").await?;
    }
    write
        .write_all(format!("MAIL FROM:<{from}>\r\n").as_bytes())
        .await
        .map_err(|e| e.to_string())?;
    expect(&mut reader, "250").await?;
    for rcpt in to {
        write
            .write_all(format!("RCPT TO:<{rcpt}>\r\n").as_bytes())
            .await
            .map_err(|e| e.to_string())?;
        expect(&mut reader, "250").await?;
    }
    write
        .write_all(b"DATA\r\n")
        .await
        .map_err(|e| e.to_string())?;
    expect(&mut reader, "354").await?;
    let mut body = Vec::with_capacity(raw.len() + 8);
    for line in raw.split_inclusive(|b| *b == b'\n') {
        if line.starts_with(b".") {
            body.push(b'.');
        }
        body.extend_from_slice(line);
    }
    if !body.ends_with(b"\n") {
        body.extend_from_slice(b"\r\n");
    }
    body.extend_from_slice(b".\r\n");
    write.write_all(&body).await.map_err(|e| e.to_string())?;
    expect(&mut reader, "250").await?;
    let _ = write.write_all(b"QUIT\r\n").await;
    Ok(())
}

/// Recipients and sender from a message's headers (what `sendmail -t` does).
pub fn envelope_of(raw: &[u8]) -> (String, Vec<String>) {
    let Some(message) = mail_parser::MessageParser::default().parse(raw) else {
        return (String::new(), Vec::new());
    };
    let collect = |l: Option<mail_parser::AddressList<'_>>| -> Vec<String> {
        l.map(|l| {
            l.mailboxes()
                .filter_map(|m| m.address().map(str::to_string))
                .collect()
        })
        .unwrap_or_default()
    };
    let mut to = collect(message.to());
    to.extend(collect(message.cc()));
    to.extend(collect(message.bcc()));
    let from = collect(message.from())
        .into_iter()
        .next()
        .unwrap_or_default();
    (from, to)
}

/// A small text + HTML message for "send a test".
pub fn test_message(from: &str, to: &str, site: Option<&str>) -> Vec<u8> {
    let subject = match site {
        Some(site) => format!("LocDev test message for {site}"),
        None => "LocDev test message".to_string(),
    };
    mail_builder::MessageBuilder::new()
        .from(from)
        .to(to)
        .subject(subject)
        .text_body("This message was sent by LocDev to its own Mailbox. Nothing left this machine.")
        .html_body("<p>This message was sent by <strong>LocDev</strong> to its own Mailbox.</p><p>Nothing left this machine.</p>")
        .write_to_vec()
        .unwrap_or_default()
}

/// An HTML message with its text alternative (Email Studio test sends).
pub fn html_message(from: &str, to: &str, subject: &str, html: &str, text: &str) -> Vec<u8> {
    let mut builder = mail_builder::MessageBuilder::new()
        .from(from)
        .to(to)
        .subject(subject)
        .html_body(html.to_string());
    if !text.trim().is_empty() {
        builder = builder.text_body(text.to_string());
    }
    builder.write_to_vec().unwrap_or_default()
}
