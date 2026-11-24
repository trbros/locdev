//! Persistent list of registered projects (`projects.toml`).

use std::path::{Path, PathBuf};

use locdev_core::model::Project;
use locdev_core::{AppError, ErrorCode};
use serde::{Deserialize, Serialize};

use crate::fsutil::write_atomic;

const REGISTRY_VERSION: u32 = 1;

#[derive(Debug, Serialize, Deserialize)]
struct RegistryFile {
    version: u32,
    #[serde(default, rename = "project")]
    projects: Vec<Project>,
}

#[derive(Debug)]
pub struct Registry {
    path: PathBuf,
    projects: Vec<Project>,
}

impl Registry {
    pub fn load(path: &Path) -> Result<Self, AppError> {
        let projects = match std::fs::read_to_string(path) {
            Ok(raw) => parse(&raw).map_err(|e| {
                AppError::new(
                    ErrorCode::Config,
                    format!("{}: {}", path.display(), e.message),
                )
            })?,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Vec::new(),
            Err(e) => return Err(e.into()),
        };

        let mut projects = projects;
        // Sites registered before paths were kept plain (verbatim on Windows).
        for project in &mut projects {
            project.path = locdev_core::paths::plain(std::mem::take(&mut project.path));
            project.parked = project.parked.take().map(locdev_core::paths::plain);
        }
        Ok(Self {
            path: path.to_path_buf(),
            projects,
        })
    }

    pub fn save(&self) -> Result<(), AppError> {
        let file = RegistryFile {
            version: REGISTRY_VERSION,
            projects: self.projects.clone(),
        };
        let raw = toml::to_string_pretty(&file)
            .map_err(|e| AppError::internal(format!("could not serialize registry: {e}")))?;

        write_atomic(&self.path, raw.as_bytes())
    }

    pub fn all(&self) -> &[Project] {
        &self.projects
    }

    pub fn get(&self, name: &str) -> Result<&Project, AppError> {
        self.projects
            .iter()
            .find(|p| p.name == name)
            .ok_or_else(|| AppError::not_found(format!("no project named '{name}'")))
    }

    pub fn get_mut(&mut self, name: &str) -> Result<&mut Project, AppError> {
        self.projects
            .iter_mut()
            .find(|p| p.name == name)
            .ok_or_else(|| AppError::not_found(format!("no project named '{name}'")))
    }

    pub fn insert(&mut self, project: Project) -> Result<(), AppError> {
        if let Some(existing) = self
            .projects
            .iter()
            .find(|p| p.name == project.name || p.path == project.path)
        {
            let reason = if existing.name == project.name {
                format!("a project named '{}' already exists", project.name)
            } else {
                format!(
                    "{} is already registered as '{}'",
                    project.path.display(),
                    existing.name
                )
            };
            return Err(AppError::new(ErrorCode::AlreadyExists, reason));
        }

        self.projects.push(project);
        self.projects.sort_by(|a, b| a.name.cmp(&b.name));
        Ok(())
    }

    pub fn remove(&mut self, name: &str) -> Result<Project, AppError> {
        let index = self
            .projects
            .iter()
            .position(|p| p.name == name)
            .ok_or_else(|| AppError::not_found(format!("no project named '{name}'")))?;

        Ok(self.projects.remove(index))
    }

    /// Ports claimed by any project, its main server or its helper.
    pub fn ports_in_use(&self) -> impl Iterator<Item = u16> + '_ {
        self.projects
            .iter()
            .flat_map(|p| [p.port, p.dev_server.as_ref().and_then(|d| d.port)])
            .flatten()
    }
}

fn parse(raw: &str) -> Result<Vec<Project>, AppError> {
    let table: toml::Table = toml::from_str(raw).map_err(|e| AppError::invalid(e.to_string()))?;

    let version = match table.get("version") {
        None => REGISTRY_VERSION,
        Some(toml::Value::Integer(v)) => u32::try_from(*v).unwrap_or(u32::MAX),
        Some(_) => return Err(AppError::invalid("'version' must be an integer")),
    };

    // Older formats get migrated here, one version at a time.
    match version {
        REGISTRY_VERSION => {}
        v if v > REGISTRY_VERSION => {
            return Err(AppError::invalid(format!(
                "written by a newer LocDev (format {v}, this build reads up to {REGISTRY_VERSION})"
            )));
        }
        v => return Err(AppError::invalid(format!("unsupported format version {v}"))),
    }

    let file: RegistryFile = table
        .try_into()
        .map_err(|e: toml::de::Error| AppError::invalid(e.to_string()))?;

    let mut projects = file.projects;
    for project in &mut projects {
        upgrade_command(project);
    }
    Ok(projects)
}

/// The Laravel command LocDev used to write, which loses injected variables
/// (`artisan serve` drops them before starting `php -S`). Only that exact
/// string is upgraded; a command the user wrote stays as it is.
const OLD_LARAVEL: &str = "php artisan serve --host=127.0.0.1 --port={port}";

fn upgrade_command(project: &mut Project) {
    if project.command.as_deref() == Some(OLD_LARAVEL) {
        project.command = Some(format!("{OLD_LARAVEL} --no-reload"));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use locdev_core::model::ProjectKind;

    fn project(name: &str, path: &str) -> Project {
        Project {
            name: name.into(),
            domain: format!("{name}.test"),
            path: path.into(),
            kind: ProjectKind::Node,
            command: Some("npm run dev".into()),
            port: Some(8100),
            auto_start: false,
            ..Default::default()
        }
    }

    #[test]
    fn round_trip() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("projects.toml");

        let mut registry = Registry::load(&file).unwrap();
        registry.insert(project("blog", "/srv/blog")).unwrap();
        registry.save().unwrap();

        let raw = std::fs::read_to_string(&file).unwrap();
        assert!(raw.starts_with("version = 1"));
        assert!(raw.contains("[[project]]"));

        let registry = Registry::load(&file).unwrap();
        assert_eq!(registry.get("blog").unwrap(), &project("blog", "/srv/blog"));
    }

    #[test]
    fn loads_legacy_windows_paths_in_the_same_form() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("projects.toml");
        let mut registry = Registry::load(&file).unwrap();
        let mut site = project("shop", r"\\?\C:\dev\shop");
        site.parked = Some(r"\\?\C:\dev".into());
        registry.insert(site).unwrap();
        registry.save().unwrap();

        let registry = Registry::load(&file).unwrap();
        let site = registry.get("shop").unwrap();
        assert_eq!(site.path, Path::new(r"C:\dev\shop"));
        assert_eq!(site.parked.as_deref(), Some(Path::new(r"C:\dev")));
    }

    #[test]
    fn rejects_duplicates() {
        let dir = tempfile::tempdir().unwrap();
        let mut registry = Registry::load(&dir.path().join("p.toml")).unwrap();

        registry.insert(project("blog", "/srv/blog")).unwrap();
        let by_name = registry.insert(project("blog", "/srv/other")).unwrap_err();
        let by_path = registry.insert(project("other", "/srv/blog")).unwrap_err();

        assert_eq!(by_name.code, ErrorCode::AlreadyExists);
        assert_eq!(by_path.code, ErrorCode::AlreadyExists);
    }

    #[test]
    fn rejects_newer_format() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("projects.toml");
        std::fs::write(&file, "version = 99\n").unwrap();

        let error = Registry::load(&file).unwrap_err();
        assert_eq!(error.code, ErrorCode::Config);
        assert!(error.message.contains("newer"));
    }

    #[test]
    fn upgrades_only_the_old_generated_laravel_command() {
        let mut generated = project("a", "/a");
        generated.command = Some(OLD_LARAVEL.into());
        upgrade_command(&mut generated);
        assert_eq!(
            generated.command.as_deref(),
            Some("php artisan serve --host=127.0.0.1 --port={port} --no-reload")
        );
        let mut custom = project("b", "/b");
        custom.command = Some("php artisan serve --port={port} --host=0.0.0.0".into());
        upgrade_command(&mut custom);
        assert_eq!(
            custom.command.as_deref(),
            Some("php artisan serve --port={port} --host=0.0.0.0")
        );
    }
}
