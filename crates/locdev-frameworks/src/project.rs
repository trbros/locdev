//! A project folder as the engine sees it: manifests and lockfiles read once
//! per analysis, lazily, with a size limit. Nothing in the project is run.

use std::cell::OnceCell;
use std::path::{Path, PathBuf};

use serde_json::Value;

use crate::catalog::Rule;

/// Manifests larger than this are ignored.
const MAX_FILE: u64 = 8 * 1024 * 1024;

pub struct ProjectFiles {
    pub root: PathBuf,
    package: OnceCell<Option<Value>>,
    composer: OnceCell<Option<Value>>,
    texts: std::cell::RefCell<std::collections::HashMap<String, Option<String>>>,
    root_names: OnceCell<Vec<String>>,
}

impl ProjectFiles {
    pub fn new(root: &Path) -> Self {
        Self {
            root: root.to_path_buf(),
            package: OnceCell::new(),
            composer: OnceCell::new(),
            texts: Default::default(),
            root_names: OnceCell::new(),
        }
    }

    pub fn has(&self, rel: &str) -> bool {
        self.root.join(rel).exists()
    }

    /// A file's text (cached; `None` when missing, too large or not UTF-8).
    pub fn text(&self, rel: &str) -> Option<String> {
        if let Some(cached) = self.texts.borrow().get(rel) {
            return cached.clone();
        }
        let path = self.root.join(rel);
        let text = std::fs::metadata(&path)
            .ok()
            .filter(|m| m.is_file() && m.len() <= MAX_FILE)
            .and_then(|_| std::fs::read_to_string(&path).ok());
        self.texts
            .borrow_mut()
            .insert(rel.to_string(), text.clone());
        text
    }

    pub fn package(&self) -> Option<&Value> {
        self.package
            .get_or_init(|| {
                self.text("package.json")
                    .and_then(|t| serde_json::from_str(&t).ok())
            })
            .as_ref()
    }

    pub fn composer(&self) -> Option<&Value> {
        self.composer
            .get_or_init(|| {
                self.text("composer.json")
                    .and_then(|t| serde_json::from_str(&t).ok())
            })
            .as_ref()
    }

    /// Names at the project root (for `*.ext` rules).
    fn root_names(&self) -> &[String] {
        self.root_names.get_or_init(|| {
            std::fs::read_dir(&self.root)
                .map(|dir| {
                    dir.filter_map(Result::ok)
                        .filter_map(|e| e.file_name().into_string().ok())
                        .collect()
                })
                .unwrap_or_default()
        })
    }

    pub fn script(&self, name: &str) -> Option<String> {
        self.package()?["scripts"][name]
            .as_str()
            .map(str::to_string)
    }

    pub fn npm_declared(&self, name: &str) -> Option<String> {
        let package = self.package()?;
        ["dependencies", "devDependencies", "peerDependencies"]
            .iter()
            .find_map(|s| package[s][name].as_str().map(str::to_string))
    }

    fn composer_declared(&self, name: &str) -> Option<String> {
        let composer = self.composer()?;
        ["require", "require-dev"]
            .iter()
            .find_map(|s| composer[s][name].as_str().map(str::to_string))
    }

    fn csproj_texts(&self) -> Vec<String> {
        self.root_names()
            .iter()
            .filter(|n| n.ends_with(".csproj") || n.ends_with(".fsproj"))
            .filter_map(|n| self.text(n))
            .collect()
    }

    fn gradle_text(&self) -> Option<String> {
        self.text("build.gradle.kts")
            .or_else(|| self.text("build.gradle"))
    }

    /// Whether `rule` holds, and the evidence to show when it does.
    pub fn check(&self, rule: &Rule) -> Option<String> {
        if let Some(file) = &rule.file {
            return self.has(file).then(|| file.clone());
        }
        if let Some(dir) = &rule.dir {
            return self.root.join(dir).is_dir().then(|| format!("{dir}/"));
        }
        if let Some(glob) = &rule.glob {
            let suffix = glob.trim_start_matches('*');
            return self
                .root_names()
                .iter()
                .find(|n| n.ends_with(suffix) && n.len() > suffix.len())
                .cloned();
        }
        if let Some(name) = &rule.npm {
            return self
                .npm_declared(name)
                .map(|_| format!("package.json: {name}"));
        }
        if let Some(text) = &rule.script {
            let scripts = self.package()?["scripts"].as_object()?;
            return scripts
                .iter()
                .find(|(_, body)| body.as_str().is_some_and(|b| b.contains(text.as_str())))
                .map(|(name, _)| format!("package.json script `{name}`"));
        }
        if let Some(name) = &rule.composer {
            return self
                .composer_declared(name)
                .map(|_| format!("composer.json: {name}"));
        }
        if let Some(name) = &rule.gem {
            let in_gemfile = self.text("Gemfile").is_some_and(|t| gemfile_has(&t, name));
            let in_lock = self
                .text("Gemfile.lock")
                .is_some_and(|t| gem_lock_version(&t, name).is_some());
            return (in_gemfile || in_lock).then(|| format!("Gemfile: {name}"));
        }
        if let Some(name) = &rule.python {
            return self.python_source(name).map(|f| format!("{f}: {name}"));
        }
        if let Some(name) = &rule.cargo {
            let toml: toml::Table = self.text("Cargo.toml")?.parse().ok()?;
            let found = ["dependencies", "dev-dependencies"]
                .iter()
                .any(|s| toml.get(*s).and_then(|d| d.get(name.as_str())).is_some());
            return found.then(|| format!("Cargo.toml: {name}"));
        }
        if let Some(module) = &rule.go {
            return self
                .text("go.mod")
                .filter(|t| {
                    t.lines().any(|l| {
                        l.trim_start().starts_with(module.as_str())
                            || l.contains(&format!("require {module}"))
                    })
                })
                .map(|_| format!("go.mod: {module}"));
        }
        if let Some(artifact) = &rule.maven {
            return self
                .text("pom.xml")
                .filter(|t| t.contains(&format!("<artifactId>{artifact}")))
                .map(|_| format!("pom.xml: {artifact}"));
        }
        if let Some(text) = &rule.gradle {
            return self
                .gradle_text()
                .filter(|t| t.contains(text.as_str()))
                .map(|_| format!("build.gradle: {text}"));
        }
        if let Some(text) = &rule.nuget {
            return self
                .csproj_texts()
                .iter()
                .any(|t| t.contains(text.as_str()))
                .then(|| format!("*.csproj: {text}"));
        }
        if let Some([file, text]) = &rule.contains {
            return self
                .text(file)
                .filter(|t| t.contains(text.as_str()))
                .map(|_| file.clone());
        }
        None
    }

    /// The version a rule points at, and where it came from.
    pub fn version(&self, rule: &Rule) -> Option<(String, String)> {
        if let Some(name) = &rule.npm {
            let installed = self
                .text(&format!("node_modules/{name}/package.json"))
                .and_then(|t| serde_json::from_str::<Value>(&t).ok())
                .and_then(|v| v["version"].as_str().map(str::to_string));
            return match installed {
                Some(v) => Some((v, "node_modules".into())),
                None => self
                    .npm_declared(name)
                    .map(|r| (r, "package.json (range)".into())),
            };
        }
        if let Some(name) = &rule.composer {
            for (file, key) in [
                ("vendor/composer/installed.json", "packages"),
                ("composer.lock", "packages"),
            ] {
                let found = self
                    .text(file)
                    .and_then(|t| serde_json::from_str::<Value>(&t).ok())
                    .and_then(|v| {
                        // installed.json is either an array (Composer 1) or { packages: [...] }.
                        let list = if v.is_array() { v } else { v[key].clone() };
                        list.as_array()?
                            .iter()
                            .find(|p| p["name"].as_str() == Some(name.as_str()))
                            .and_then(|p| {
                                p["version"]
                                    .as_str()
                                    .map(|s| s.trim_start_matches('v').to_string())
                            })
                    });
                if let Some(v) = found {
                    return Some((v, file.into()));
                }
            }
            return self
                .composer_declared(name)
                .map(|r| (r, "composer.json (range)".into()));
        }
        if let Some(name) = &rule.gem {
            return self
                .text("Gemfile.lock")
                .and_then(|t| gem_lock_version(&t, name))
                .map(|v| (v, "Gemfile.lock".into()));
        }
        if let Some(name) = &rule.python {
            for lock in ["uv.lock", "poetry.lock"] {
                if let Some(v) = self.text(lock).and_then(|t| toml_lock_version(&t, name)) {
                    return Some((v, lock.into()));
                }
            }
            for file in [
                "requirements.txt",
                "requirements/base.txt",
                "requirements/dev.txt",
            ] {
                if let Some(v) = self.text(file).and_then(|t| requirement_pin(&t, name)) {
                    return Some((v, file.into()));
                }
            }
            return None;
        }
        if let Some(name) = &rule.cargo {
            return self
                .text("Cargo.lock")
                .and_then(|t| toml_lock_version(&t, name))
                .map(|v| (v, "Cargo.lock".into()));
        }
        if let Some(module) = &rule.go {
            let text = self.text("go.mod")?;
            return text
                .lines()
                .map(|l| l.trim().trim_start_matches("require").trim())
                .find_map(|l| {
                    l.strip_prefix(module.as_str())?
                        .split_whitespace()
                        .next()
                        .map(str::to_string)
                })
                .map(|v| (v.trim_start_matches('v').to_string(), "go.mod".into()));
        }
        if let Some(artifact) = &rule.maven {
            let text = self.text("pom.xml")?;
            let at = text.find(&format!("<artifactId>{artifact}</artifactId>"))?;
            let rest = &text[at..text.len().min(at + 400)];
            let start = rest.find("<version>")? + "<version>".len();
            let end = rest[start..].find("</version>")?;
            return Some((
                rest[start..start + end].trim().to_string(),
                "pom.xml".into(),
            ));
        }
        if let Some(plugin) = &rule.gradle {
            let text = self.gradle_text()?;
            let at = text.find(plugin.as_str())?;
            let rest = &text[at..text.len().min(at + 200)];
            let after = &rest[rest.find("version")? + "version".len()..];
            let quote = after.find(['"', '\''])?;
            let value = &after[quote + 1..];
            let end = value.find(['"', '\''])?;
            return Some((value[..end].to_string(), "build.gradle".into()));
        }
        if rule.nuget.as_deref() == Some("TargetFramework") {
            return self.csproj_texts().iter().find_map(|t| {
                let start = t.find("<TargetFramework>")? + "<TargetFramework>".len();
                let end = t[start..].find('<')?;
                let tfm = &t[start..start + end];
                Some((
                    tfm.trim_start_matches("net").to_string(),
                    "*.csproj (TargetFramework)".into(),
                ))
            });
        }
        None
    }

    /// The Django project package (`shop` from `"shop.settings"` in manage.py).
    pub fn django_module(&self) -> Option<String> {
        let text = self.text("manage.py")?;
        let at = text.find("DJANGO_SETTINGS_MODULE")?;
        let rest = &text[at..text.len().min(at + 200)];
        // The value is the string after the comma: ('DJANGO_SETTINGS_MODULE', 'x.settings').
        let after = &rest[rest.find(',')? + 1..];
        let quote = after.find(['"', '\''])?;
        let value = &after[quote + 1..];
        let module = value.split(['"', '\'']).next()?.strip_suffix(".settings")?;
        (!module.is_empty()
            && module
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '.'))
        .then(|| module.to_string())
    }

    /// The Python file that declares `name`, if any.
    fn python_source(&self, name: &str) -> Option<&'static str> {
        let wanted = normalize_python(name);
        let in_requirements = |text: &str| {
            text.lines().any(|line| {
                let line = line.split('#').next().unwrap_or_default().trim();
                let pkg: String = line
                    .chars()
                    .take_while(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '.'))
                    .collect();
                !pkg.is_empty() && normalize_python(&pkg) == wanted
            })
        };
        for file in [
            "requirements.txt",
            "requirements/base.txt",
            "requirements/dev.txt",
            "requirements-dev.txt",
        ] {
            if self.text(file).is_some_and(|t| in_requirements(&t)) {
                return Some(match file {
                    "requirements.txt" => "requirements.txt",
                    _ => "requirements",
                });
            }
        }
        if let Some(toml) = self
            .text("pyproject.toml")
            .and_then(|t| t.parse::<toml::Table>().ok())
        {
            let listed = toml
                .get("project")
                .and_then(|p| p.get("dependencies"))
                .and_then(|d| d.as_array())
                .is_some_and(|deps| deps.iter().filter_map(|d| d.as_str()).any(in_requirements));
            let poetry = toml
                .get("tool")
                .and_then(|t| t.get("poetry"))
                .and_then(|p| p.get("dependencies"))
                .and_then(|d| d.as_table())
                .is_some_and(|deps| deps.keys().any(|k| normalize_python(k) == wanted));
            if listed || poetry {
                return Some("pyproject.toml");
            }
        }
        if let Some(pipfile) = self
            .text("Pipfile")
            .and_then(|t| t.parse::<toml::Table>().ok())
            && pipfile
                .get("packages")
                .and_then(|p| p.as_table())
                .is_some_and(|deps| deps.keys().any(|k| normalize_python(k) == wanted))
        {
            return Some("Pipfile");
        }
        None
    }
}

/// PEP 503: case-insensitive, runs of `-_.` are equivalent.
fn normalize_python(name: &str) -> String {
    let mut out = String::new();
    for c in name.chars() {
        let c = if matches!(c, '_' | '.') {
            '-'
        } else {
            c.to_ascii_lowercase()
        };
        if !(c == '-' && out.ends_with('-')) {
            out.push(c);
        }
    }
    out
}

fn gemfile_has(text: &str, name: &str) -> bool {
    text.lines().any(|line| {
        let line = line.trim();
        line.starts_with(&format!("gem \"{name}\"")) || line.starts_with(&format!("gem '{name}'"))
    })
}

/// `    rails (8.0.1)` under `specs:` in Gemfile.lock.
fn gem_lock_version(text: &str, name: &str) -> Option<String> {
    let prefix = format!("    {name} (");
    text.lines()
        .find_map(|l| l.strip_prefix(&prefix))
        .and_then(|rest| rest.split(')').next())
        .map(str::to_string)
}

/// `name = "x"` followed by `version = "…"` (Cargo.lock, poetry.lock, uv.lock).
fn toml_lock_version(text: &str, name: &str) -> Option<String> {
    let wanted = normalize_python(name);
    let mut lines = text.lines();
    while let Some(line) = lines.next() {
        let Some(found) = line
            .strip_prefix("name = \"")
            .and_then(|r| r.strip_suffix('"'))
        else {
            continue;
        };
        if normalize_python(found) == wanted {
            return lines
                .next()
                .and_then(|l| l.strip_prefix("version = \""))
                .and_then(|r| r.strip_suffix('"'))
                .map(str::to_string);
        }
    }
    None
}

/// `django==5.1.2` in a requirements file.
fn requirement_pin(text: &str, name: &str) -> Option<String> {
    let wanted = normalize_python(name);
    text.lines().find_map(|line| {
        let (pkg, version) = line.split('#').next()?.trim().split_once("==")?;
        let pkg = pkg.split('[').next()?.trim();
        (normalize_python(pkg) == wanted).then(|| version.trim().to_string())
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn python_names_normalize() {
        assert_eq!(normalize_python("Django_Environ"), "django-environ");
        assert_eq!(normalize_python("python.dotenv"), "python-dotenv");
    }

    #[test]
    fn lock_readers() {
        let gem = "GEM\n  specs:\n    rails (8.0.1)\n      actionpack (= 8.0.1)\n";
        assert_eq!(gem_lock_version(gem, "rails").as_deref(), Some("8.0.1"));
        assert_eq!(gem_lock_version(gem, "actionpack"), None);
        let lock = "[[package]]\nname = \"Django\"\nversion = \"5.1.2\"\n";
        assert_eq!(toml_lock_version(lock, "django").as_deref(), Some("5.1.2"));
        assert_eq!(
            requirement_pin("flask[async]==3.0.3\n", "flask").as_deref(),
            Some("3.0.3")
        );
    }
}
