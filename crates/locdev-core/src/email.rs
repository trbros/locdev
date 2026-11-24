//! Email Studio as clients see it: the project's email templates and
//! components, and a template as editable segments.

use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EmailFormat {
    /// A Blade view rendered as HTML.
    Blade,
    /// A Laravel Markdown mailable (`<x-mail::message>`).
    Markdown,
    /// A plain-text Blade view.
    Text,
    Html,
    /// Formats LocDev shows and edits as code only.
    Mjml,
    React,
    Erb,
    Django,
}

impl EmailFormat {
    /// Whether LocDev's visual blocks can be used in it.
    pub fn visual(self) -> bool {
        matches!(self, Self::Blade | Self::Markdown | Self::Html)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MailableInfo {
    /// `App\Mail\OrderShipped`.
    pub class: String,
    pub path: String,
    /// Variables the template gets (constructor properties, `with:` keys).
    pub variables: Vec<String>,
    #[serde(default)]
    pub subject: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EmailTemplate {
    /// Relative to the project root.
    pub path: String,
    /// `emails.welcome` for a Blade view, else the file name.
    pub name: String,
    pub format: EmailFormat,
    #[serde(default)]
    pub mailables: Vec<MailableInfo>,
    /// Has LocDev blocks already.
    pub has_blocks: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ComponentProp {
    pub name: String,
    /// The default as written in the source (`'primary'`), if any.
    #[serde(default)]
    pub default: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EmailComponent {
    pub name: String,
    pub path: String,
    /// How a template uses it (`<x-mail.button>`).
    pub usage: String,
    pub props: Vec<ComponentProp>,
    /// Takes a body (`{{ $slot }}`).
    pub slot: bool,
    #[serde(default)]
    pub description: Option<String>,
    /// From the framework itself (Laravel's `mail::button`), not the project.
    #[serde(default)]
    pub builtin: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EmailCatalog {
    pub framework: String,
    pub templates: Vec<EmailTemplate>,
    pub components: Vec<EmailComponent>,
    /// Where `locdev new email` puts things, and the native command it uses.
    pub conventions: Vec<String>,
    /// The framework's own generators, when available (`php artisan make:mail`).
    pub native: Vec<String>,
    /// Framework preview available (Laravel with vendor/ installed).
    pub framework_preview: bool,
}

/// A part of a template: a LocDev block, or code kept exactly as written.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Segment {
    Block {
        id: String,
        /// `heading`, `text`, `button`, `image`, `divider`, `spacer`, `columns`, `header`, `footer`, `component`.
        block: String,
        props: Value,
        /// The generated markup was changed outside LocDev: kept as written
        /// until the block is explicitly regenerated from its properties.
        #[serde(default)]
        edited: bool,
    },
    Code {
        id: String,
        code: String,
    },
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EmailDoc {
    pub path: String,
    pub format: EmailFormat,
    pub content: String,
    pub hash: String,
    pub segments: Vec<Segment>,
    /// Width, colors, font: what blocks render with.
    pub theme: Value,
    /// Variables the template uses (`$user`, `$url`), for sample data.
    pub variables: Vec<String>,
    /// Sample data kept by LocDev for previews.
    pub sample: Value,
    #[serde(default)]
    pub notes: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PreviewMode {
    /// LocDev substitutes sample data; Blade logic isn't evaluated.
    Static,
    /// The framework renders it, in a separate process.
    Framework,
}

#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct EmailPreview {
    pub html: String,
    #[serde(default)]
    pub text: Option<String>,
    /// The file content the segments produce (what saving would write).
    pub content: String,
    #[serde(default)]
    pub notes: Vec<String>,
    #[serde(default)]
    pub error: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EmailCreated {
    pub files: Vec<String>,
    /// The command LocDev ran, when it used the framework's own generator.
    #[serde(default)]
    pub ran: Option<String>,
    pub notes: Vec<String>,
}
