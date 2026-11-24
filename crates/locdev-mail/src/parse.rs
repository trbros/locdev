//! What a captured message contains, read with mail-parser. Only facts the
//! message or the SMTP session carry; problems are checks against RFC 5322
//! and MIME that can be pointed at.

use locdev_core::mail::{MailAttachment, MailHeader, MailLink};
use mail_parser::{Address, MessageParser, PartKind};

pub struct Parsed {
    pub from: String,
    pub to: Vec<String>,
    pub cc: Vec<String>,
    pub reply_to: Vec<String>,
    pub subject: String,
    pub date: Option<u64>,
    pub html: Option<String>,
    pub text: Option<String>,
    pub text_generated: bool,
    pub headers: Vec<MailHeader>,
    pub attachments: Vec<MailAttachment>,
    pub links: Vec<MailLink>,
    pub problems: Vec<String>,
    pub preview: String,
}

fn addresses(list: Option<mail_parser::AddressList<'_>>) -> Vec<String> {
    let Some(list) = list else { return Vec::new() };
    list.iter()
        .flat_map(|a| match a {
            Address::Mailbox(m) => vec![m],
            Address::Group(g) => g.mailboxes().collect(),
        })
        .map(|m| match (m.name(), m.address()) {
            (Some(name), Some(addr)) => format!("{name} <{addr}>"),
            (None, Some(addr)) => addr.to_string(),
            (Some(name), None) => name.to_string(),
            (None, None) => String::new(),
        })
        .filter(|s| !s.is_empty())
        .collect()
}

/// `href="…"` targets and their text, in order, without duplicates.
pub fn html_links(html: &str) -> Vec<MailLink> {
    let lower = html.to_ascii_lowercase();
    let mut out: Vec<MailLink> = Vec::new();
    let mut at = 0;
    while let Some(pos) = lower[at..].find("<a ") {
        let start = at + pos;
        let Some(end) = lower[start..].find('>').map(|e| start + e) else {
            break;
        };
        let tag = &html[start..end];
        let tag_lower = &lower[start..end];
        let href = tag_lower.find("href=").and_then(|h| {
            let rest = &tag[h + 5..];
            let quote = rest.chars().next()?;
            if quote == '"' || quote == '\'' {
                rest[1..].find(quote).map(|e| rest[1..1 + e].to_string())
            } else {
                Some(
                    rest.split(|c: char| c.is_whitespace() || c == '>')
                        .next()
                        .unwrap_or("")
                        .to_string(),
                )
            }
        });
        let close = lower[end..].find("</a>").map(|c| end + c);
        let text = close
            .map(|c| strip_tags(&html[end + 1..c]))
            .filter(|t| !t.is_empty());
        if let Some(url) = href
            .map(|u| u.replace("&amp;", "&"))
            .filter(|u| !u.is_empty())
            && !out.iter().any(|l| l.url == url)
        {
            out.push(MailLink { url, text });
        }
        at = close.unwrap_or(end) + 1;
        if at >= html.len() {
            break;
        }
    }
    out
}

fn strip_tags(s: &str) -> String {
    let mut out = String::new();
    let mut in_tag = false;
    for c in s.chars() {
        match c {
            '<' => in_tag = true,
            '>' => in_tag = false,
            c if !in_tag => out.push(c),
            _ => {}
        }
    }
    out.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// `http(s)://` URLs in plain text.
fn text_links(text: &str) -> Vec<MailLink> {
    let mut out: Vec<MailLink> = Vec::new();
    for word in text.split_whitespace() {
        if let Some(i) = word.find("http://").or_else(|| word.find("https://")) {
            let url = word[i..]
                .trim_end_matches(['.', ',', ')', '>', ']', '"', '\''])
                .to_string();
            if !out.iter().any(|l| l.url == url) {
                out.push(MailLink { url, text: None });
            }
        }
    }
    out
}

fn header_text(raw: &[u8]) -> String {
    // Unfold continuation lines for display.
    String::from_utf8_lossy(raw)
        .replace("\r\n", "\n")
        .replace("\n ", " ")
        .replace("\n\t", " ")
        .trim()
        .to_string()
}

pub fn parse(raw: &[u8]) -> Parsed {
    let mut problems = Vec::new();
    let Some(message) = MessageParser::default().parse(raw) else {
        return Parsed {
            from: String::new(),
            to: Vec::new(),
            cc: Vec::new(),
            reply_to: Vec::new(),
            subject: String::new(),
            date: None,
            html: None,
            text: Some(String::from_utf8_lossy(raw).into_owned()),
            text_generated: false,
            headers: Vec::new(),
            attachments: Vec::new(),
            links: Vec::new(),
            problems: vec!["the message has no header block LocDev could read".into()],
            preview: String::new(),
        };
    };

    let headers: Vec<MailHeader> = message
        .headers()
        .iter()
        .map(|h| MailHeader {
            name: h.raw_name().to_string(),
            value: header_text(h.raw_value()),
        })
        .collect();
    let has = |name: &str| headers.iter().any(|h| h.name.eq_ignore_ascii_case(name));

    if !has("From") {
        problems.push("no From header (RFC 5322 §3.6 requires one)".into());
    }
    if !has("Date") {
        problems.push("no Date header (RFC 5322 §3.6 requires one)".into());
    } else if message.date().is_none() {
        problems.push("the Date header doesn't parse as an RFC 5322 date".into());
    }
    if !has("Message-ID") {
        problems.push(
            "no Message-ID header (recommended by RFC 5322 §3.6.4; spam filters penalize it)"
                .into(),
        );
    }
    if !has("Subject") {
        problems.push("no Subject header".into());
    }
    if headers
        .iter()
        .filter(|h| h.name.eq_ignore_ascii_case("From"))
        .count()
        > 1
    {
        problems.push("more than one From header".into());
    }
    if raw.split(|b| *b == b'\n').any(|line| line.len() > 999) {
        problems.push("a line is longer than 998 characters (RFC 5322 §2.1.1)".into());
    }
    let mime = has("MIME-Version");
    if !mime && raw.iter().any(|b| *b >= 0x80) {
        problems
            .push("8-bit content without MIME-Version/Content-Type: clients may garble it".into());
    }
    for part in message.parts() {
        let (_, decode) = part.decoded_checked();
        for name in decode.names() {
            problems.push(format!("a part has {}", name.replace('_', " ")));
        }
    }

    let html = message
        .body_html(0)
        .filter(|_| {
            message
                .html_body()
                .any(|p| matches!(p.kind(), PartKind::Html))
        })
        .map(|h| h.into_owned());
    let real_text = message
        .text_body()
        .any(|p| matches!(p.kind(), PartKind::Text));
    let text = message.body_text(0).map(|t| t.into_owned());
    if html.is_some() && !real_text {
        problems.push(
            "HTML only, no plain-text alternative (text-only clients and spam filters prefer both)"
                .into(),
        );
    }

    let attachments: Vec<MailAttachment> = files(&message)
        .into_iter()
        .enumerate()
        .map(|(index, part)| MailAttachment {
            index,
            name: part.attachment_name().map(str::to_string),
            content_type: part
                .content_type()
                .map(|c| match c.subtype() {
                    Some(sub) => format!("{}/{sub}", c.ctype()),
                    None => c.ctype().to_string(),
                })
                .unwrap_or_else(|| "application/octet-stream".into()),
            size: part.decoded_len() as u64,
            content_id: part.content_id().map(str::to_string),
            inline: part.is_inline(),
        })
        .collect();
    if let Some(html) = &html {
        for cid in html
            .match_indices("cid:")
            .filter_map(|(i, _)| html[i + 4..].split(['"', '\'', ')', ' ']).next())
        {
            if !attachments
                .iter()
                .any(|a| a.content_id.as_deref() == Some(cid))
            {
                problems.push(format!(
                    "the HTML references cid:{cid}, which no part carries"
                ));
            }
        }
    }

    let mut links = html.as_deref().map(html_links).unwrap_or_default();
    if links.is_empty() {
        links = text.as_deref().map(text_links).unwrap_or_default();
    }
    let preview: String = text
        .as_deref()
        .unwrap_or("")
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .chars()
        .take(160)
        .collect();

    Parsed {
        from: addresses(message.from()).join(", "),
        to: addresses(message.to()),
        cc: addresses(message.cc()),
        reply_to: addresses(message.reply_to()),
        subject: message.subject().unwrap_or("").to_string(),
        date: message
            .date()
            .map(|d| (d.to_timestamp().max(0) as u64) * 1000),
        text_generated: !real_text && text.is_some(),
        html,
        text,
        headers,
        attachments,
        links,
        problems,
        preview,
    }
}

/// Every file part: attachments and inline images alike (an inline image can
/// sit in the HTML body list), in message order. Their position is the index.
fn files<'m>(message: &'m mail_parser::Message<'m>) -> Vec<mail_parser::MessagePart<'m>> {
    message
        .parts()
        .filter(|p| {
            matches!(
                p.kind(),
                PartKind::Binary | PartKind::InlineBinary | PartKind::Message(_)
            ) || p.is_attachment()
        })
        .filter(|p| !p.is_multipart())
        .collect()
}

/// The decoded bytes of file `index` (as `parse` numbers them).
pub fn attachment(raw: &[u8], index: usize) -> Option<(Option<String>, String, Vec<u8>)> {
    let message = MessageParser::default().parse(raw)?;
    let part = files(&message).into_iter().nth(index)?;
    let ctype = part
        .content_type()
        .map(|c| match c.subtype() {
            Some(sub) => format!("{}/{sub}", c.ctype()),
            None => c.ctype().to_string(),
        })
        .unwrap_or_else(|| "application/octet-stream".into());
    Some((
        part.attachment_name().map(str::to_string),
        ctype,
        part.decoded().into_owned(),
    ))
}

/// The HTML with `cid:` images inlined as `data:` URIs, so a sandboxed
/// viewer with no network access can show them.
pub fn inline_cids(raw: &[u8], html: &str) -> String {
    let Some(message) = MessageParser::default().parse(raw) else {
        return html.to_string();
    };
    let mut out = html.to_string();
    for part in files(&message) {
        let (Some(cid), Some(ct)) = (part.content_id(), part.content_type()) else {
            continue;
        };
        if ct.ctype() != "image" {
            continue;
        }
        let data = base64(&part.decoded());
        let uri = format!("data:image/{};base64,{data}", ct.subtype().unwrap_or("png"));
        out = out.replace(&format!("cid:{cid}"), &uri);
    }
    out
}

pub fn base64(bytes: &[u8]) -> String {
    const TABLE: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::with_capacity(bytes.len().div_ceil(3) * 4);
    for chunk in bytes.chunks(3) {
        let b = [
            chunk[0],
            *chunk.get(1).unwrap_or(&0),
            *chunk.get(2).unwrap_or(&0),
        ];
        let n = (u32::from(b[0]) << 16) | (u32::from(b[1]) << 8) | u32::from(b[2]);
        out.push(TABLE[(n >> 18) as usize & 63] as char);
        out.push(TABLE[(n >> 12) as usize & 63] as char);
        out.push(if chunk.len() > 1 {
            TABLE[(n >> 6) as usize & 63] as char
        } else {
            '='
        });
        out.push(if chunk.len() > 2 {
            TABLE[n as usize & 63] as char
        } else {
            '='
        });
    }
    out
}

pub fn decode_base64(input: &str) -> Option<Vec<u8>> {
    const TABLE: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = Vec::with_capacity(input.len() * 3 / 4);
    let (mut buf, mut bits) = (0u32, 0);
    for c in input.bytes().filter(|c| !matches!(c, b'=' | b'\r' | b'\n')) {
        buf = (buf << 6) | TABLE.iter().position(|t| *t == c)? as u32;
        bits += 6;
        if bits >= 8 {
            bits -= 8;
            out.push((buf >> bits) as u8);
        }
    }
    Some(out)
}
