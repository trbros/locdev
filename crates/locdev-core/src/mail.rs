//! The Mailbox as clients see it: messages LocDev's SMTP server captured,
//! what they contain, and how a site is wired to send there.

use serde::{Deserialize, Serialize};

/// What the SMTP session itself told us (not the message headers).
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct MailEnvelope {
    /// `MAIL FROM`.
    pub from: String,
    /// `RCPT TO`, in order.
    pub to: Vec<String>,
    /// The client's `EHLO`/`HELO` name.
    #[serde(default)]
    pub helo: Option<String>,
    /// The `AUTH` user, which LocDev sets to the site's name.
    #[serde(default)]
    pub auth_user: Option<String>,
    /// `ESMTP` or `SMTP`, and `+AUTH` when authenticated.
    pub protocol: String,
    pub peer: String,
    /// Unix milliseconds when the message was accepted.
    pub received_at: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MailSummary {
    pub id: String,
    /// The site it came from, when the session identified one.
    #[serde(default)]
    pub site: Option<String>,
    pub from: String,
    pub to: Vec<String>,
    pub subject: String,
    /// The `Date` header (when it parses), Unix milliseconds.
    #[serde(default)]
    pub date: Option<u64>,
    pub received_at: u64,
    pub size: u64,
    pub attachments: usize,
    pub has_html: bool,
    pub preview: String,
    #[serde(default)]
    pub read: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MailHeader {
    pub name: String,
    pub value: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MailAttachment {
    pub index: usize,
    #[serde(default)]
    pub name: Option<String>,
    pub content_type: String,
    pub size: u64,
    /// Referenced from the HTML as `cid:` (inline image).
    #[serde(default)]
    pub content_id: Option<String>,
    pub inline: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MailLink {
    pub url: String,
    /// The link text in the HTML, when there is one.
    #[serde(default)]
    pub text: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MailDetail {
    pub summary: MailSummary,
    pub envelope: MailEnvelope,
    pub headers: Vec<MailHeader>,
    #[serde(default)]
    pub cc: Vec<String>,
    #[serde(default)]
    pub reply_to: Vec<String>,
    #[serde(default)]
    pub html: Option<String>,
    #[serde(default)]
    pub text: Option<String>,
    /// The text part was generated from the HTML (the message had none).
    #[serde(default)]
    pub text_generated: bool,
    pub attachments: Vec<MailAttachment>,
    pub links: Vec<MailLink>,
    /// Formatting problems found in the message, each one checkable.
    pub problems: Vec<String>,
    /// The sending site's framework (from LocDev's stack detection).
    #[serde(default)]
    pub framework: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct MailboxStatus {
    pub enabled: bool,
    #[serde(default)]
    pub port: Option<u16>,
    #[serde(default)]
    pub error: Option<String>,
    pub messages: usize,
    pub unread: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MailWiring {
    /// LocDev puts the variables in the site's process environment.
    Automatic,
    /// The framework reads SMTP settings from code; the snippet shows what to add.
    Manual,
}

/// How a site sends mail now and what turning the Mailbox on would change.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MailSetup {
    pub site: String,
    pub framework: String,
    pub enabled: bool,
    pub wiring: MailWiring,
    /// What the project's own files say now (`MAIL_MAILER=smtp`, `MAIL_HOST=smtp.mailgun.org`).
    pub current: Vec<String>,
    /// The project points at a real mail server: turning this on needs `override`.
    pub real_smtp: bool,
    /// Variables LocDev sets in the process (values shown; the password is per site).
    pub injects: Vec<MailHeader>,
    /// For manual wiring: what to put in the project's settings.
    #[serde(default)]
    pub snippet: Option<String>,
    pub notes: Vec<String>,
}
