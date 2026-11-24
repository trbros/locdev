//! The deploy assistant as clients see it: what the project needs from a
//! host, which platforms fit and why, and readiness checks. Nothing here is
//! sent anywhere; variables travel by name only, never with values.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Rendering {
    /// Prebuilt files only (Hugo, `output: 'export'`, Astro static).
    Static,
    /// A client-side app served as static files (Vite + React).
    Spa,
    /// Server rendering by a JS framework (Next.js, Nuxt, SvelteKit…).
    Ssr,
    /// A long-running application server (Laravel, Django, Express…).
    Server,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Need {
    /// `websockets`, `workers`, `scheduler`, `postgres`, `redis`, `sqlite`, `storage`…
    pub id: String,
    pub label: String,
    /// Where LocDev saw it (`laravel/reverb in composer.json`).
    pub evidence: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EnvNeed {
    pub name: String,
    pub secret: bool,
    /// What to do with it: `set it`, `provided by the managed database`, `use the production URL`.
    pub note: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DeployProfile {
    pub framework: String,
    #[serde(default)]
    pub version: Option<String>,
    /// `node`, `php`, `python`, `ruby`, `go`, `java`, `dotnet`, `rust`, `deno`, `static`.
    pub runtime: String,
    #[serde(default)]
    pub runtime_version: Option<String>,
    pub rendering: Rendering,
    /// Why the rendering was decided this way.
    pub rendering_evidence: String,
    pub needs: Vec<Need>,
    #[serde(default)]
    pub build_command: Option<String>,
    #[serde(default)]
    pub start_command: Option<String>,
    #[serde(default)]
    pub output_dir: Option<String>,
    pub env: Vec<EnvNeed>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Verdict {
    Fits,
    NeedsChanges,
    NotSuitable,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Point {
    pub text: String,
    #[serde(default)]
    pub source: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProviderFit {
    pub id: String,
    pub name: String,
    /// `serverless`, `edge`, `containers`, `vm`, `managed servers`, `your server`, `static`.
    pub model: String,
    pub verdict: Verdict,
    pub summary: String,
    pub strengths: Vec<Point>,
    pub limitations: Vec<Point>,
    /// What this project would have to do or add on this platform.
    pub requirements: Vec<Point>,
    pub docs: String,
    /// A config file LocDev can write for it (`render.yaml`), if any.
    #[serde(default)]
    pub config_file: Option<String>,
    /// How the platform sets itself up otherwise (`fly launch`).
    #[serde(default)]
    pub setup: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CheckLevel {
    Ok,
    Warn,
    Fail,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReadinessCheck {
    pub id: String,
    pub level: CheckLevel,
    pub title: String,
    pub detail: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DeployReport {
    pub profile: DeployProfile,
    /// Best fit first; never ordered by popularity.
    pub providers: Vec<ProviderFit>,
    pub checks: Vec<ReadinessCheck>,
    /// When the platform facts were last reviewed against their docs.
    pub reviewed: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DeployConfig {
    pub provider: String,
    /// Relative to the project root.
    pub file: String,
    pub content: String,
    /// SHA-256 of `content`; writing must name it.
    pub hash: String,
    /// The file is already there: LocDev never overwrites it.
    pub exists: bool,
    /// What to check or fill in after writing.
    pub notes: Vec<String>,
}
