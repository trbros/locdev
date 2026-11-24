//! Definitions as clients see them (`locdev stack list`, the app, the docs).

use locdev_core::frameworks::DefinitionInfo;
use locdev_core::model::{ComponentCategory, Integration};

use crate::catalog::Definition;
use crate::engine::PackageManager;

pub fn info(def: &Definition) -> DefinitionInfo {
    let dev: Vec<String> = def
        .dev
        .iter()
        .map(|v| match (&v.command, v.script.first()) {
            (Some(command), _) => command.clone(),
            (None, Some(script)) => {
                let script = format!("<pm> run {script}");
                match &v.args {
                    Some(args) => format!("{script} {args}"),
                    None => script,
                }
            }
            (None, None) => String::new(),
        })
        .collect();
    let integration = if def.dev.iter().any(|v| v.command.is_some()) {
        Integration::Native
    } else if !def.dev.is_empty() {
        Integration::Script
    } else if def.detect.is_empty()
        || matches!(
            def.category,
            ComponentCategory::Ui | ComponentCategory::Tool
        )
    {
        // UI libraries and tools run through another component (Vite, Next…).
        Integration::None
    } else {
        Integration::Custom
    };
    DefinitionInfo {
        id: def.id.clone(),
        name: def.name.clone(),
        category: def.category,
        integration,
        docs: def.docs.clone(),
        runtimes: def.runtimes.clone(),
        requires: def.requires.clone(),
        dev,
        install: def.install.clone(),
        build: def.build.clone(),
        test: def.test.clone(),
        supported: def.supported.clone(),
        config_files: def.config_files.clone(),
        services: def.services.clone(),
        notes: def.notes.clone(),
    }
}

/// `{pm}` in a display command, with the project's package manager.
pub fn with_pm(text: &Option<String>, pm: Option<PackageManager>) -> Option<String> {
    let text = text.as_ref()?;
    Some(text.replace("{pm}", pm.map(PackageManager::program).unwrap_or("npm")))
}
