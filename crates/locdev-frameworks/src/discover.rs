//! Finding projects inside a parked folder (`locdev park`): the folder's
//! subfolders, down to a depth, that the detection engine recognizes as a
//! runnable project. Only reads files, like the rest of this crate.

use std::collections::HashSet;
use std::path::{Path, PathBuf};

use locdev_core::model::ProjectKind;
use locdev_core::park::glob_matches;
use locdev_core::slugify;

use crate::{Catalog, Overrides, analyze};

/// Folders that are never projects of their own: dependencies, build
/// output, caches and virtual environments.
pub const SKIPPED: &[&str] = &[
    "node_modules",
    "vendor",
    "bower_components",
    "target",
    "dist",
    "build",
    "out",
    "coverage",
    "tmp",
    "temp",
    "cache",
    "storage",
    "venv",
    "env",
    "__pycache__",
    "site-packages",
    "Pods",
    "DerivedData",
];

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Found {
    /// Canonical, in the plain form used by registered projects on Windows.
    pub path: PathBuf,
    /// Relative to the parked folder, with `/`.
    pub rel: String,
    /// The folder's name as a site name (may be empty for odd names).
    pub name: String,
    pub kind: ProjectKind,
    pub framework: String,
}

#[derive(Debug, Default)]
pub struct Discovery {
    pub projects: Vec<Found>,
    /// Folders that could not be read, and why.
    pub errors: Vec<(PathBuf, String)>,
    /// The parked folder and the non-project folders searched under it:
    /// where a new project can appear.
    pub searched: Vec<PathBuf>,
}

/// Whether the folder itself is a project (and so shouldn't be parked).
pub fn is_project(catalog: &Catalog, path: &Path) -> Option<String> {
    let stack = analyze(catalog, path, Overrides::default());
    stack.adapter.is_some().then_some(stack.framework)
}

pub fn discover(catalog: &Catalog, root: &Path, depth: u8, exclude: &[String]) -> Discovery {
    let mut out = Discovery::default();
    let mut seen = HashSet::new();
    if let Ok(canonical) = root.canonicalize() {
        seen.insert(canonical);
    }
    walk(
        catalog,
        root,
        root,
        depth.max(1),
        exclude,
        &mut seen,
        &mut out,
    );
    out.projects.sort_by(|a, b| a.rel.cmp(&b.rel));
    out
}

fn walk(
    catalog: &Catalog,
    root: &Path,
    dir: &Path,
    levels: u8,
    exclude: &[String],
    seen: &mut HashSet<PathBuf>,
    out: &mut Discovery,
) {
    out.searched.push(dir.to_path_buf());
    let entries = match std::fs::read_dir(dir) {
        Ok(entries) => entries,
        Err(error) => {
            out.errors.push((dir.to_path_buf(), error.to_string()));
            return;
        }
    };
    let mut children: Vec<PathBuf> = entries.flatten().map(|e| e.path()).collect();
    children.sort();

    for child in children {
        let Some(name) = child.file_name().map(|n| n.to_string_lossy().into_owned()) else {
            continue;
        };
        if name.starts_with('.') || SKIPPED.contains(&name.as_str()) {
            continue;
        }
        let rel = relative(root, &child);
        if exclude
            .iter()
            .any(|p| glob_matches(p, &name) || glob_matches(p, &rel))
        {
            continue;
        }
        // Follows symlinks; the canonical path guards against loops and
        // against the same project reached twice.
        if !std::fs::metadata(&child).is_ok_and(|m| m.is_dir()) {
            continue;
        }
        let canonical = match child.canonicalize() {
            Ok(path) => path,
            Err(error) => {
                out.errors.push((child, error.to_string()));
                continue;
            }
        };
        if !seen.insert(canonical.clone()) {
            continue;
        }
        let stack = analyze(catalog, &child, Overrides::default());
        if stack.adapter.is_some() {
            out.projects.push(Found {
                path: locdev_core::paths::plain(canonical),
                rel,
                name: slugify(&name),
                kind: stack.kind,
                framework: stack.framework,
            });
        } else if levels > 1 {
            walk(catalog, root, &child, levels - 1, exclude, seen, out);
        }
    }
}

fn relative(root: &Path, path: &Path) -> String {
    path.strip_prefix(root)
        .unwrap_or(path)
        .components()
        .map(|c| c.as_os_str().to_string_lossy())
        .collect::<Vec<_>>()
        .join("/")
}

#[cfg(test)]
mod tests {
    use std::fs;

    use super::*;

    fn tree(files: &[&str]) -> tempfile::TempDir {
        let dir = tempfile::tempdir().unwrap();
        for file in files {
            let path = dir.path().join(file);
            fs::create_dir_all(path.parent().unwrap()).unwrap();
            let contents = if file.ends_with("package.json") {
                r#"{"scripts":{"dev":"vite"},"devDependencies":{"vite":"^7"}}"#
            } else {
                ""
            };
            fs::write(path, contents).unwrap();
        }
        dir
    }

    fn rels(d: &Discovery) -> Vec<&str> {
        d.projects.iter().map(|p| p.rel.as_str()).collect()
    }

    #[test]
    fn finds_projects_by_their_files_not_their_names() {
        let dir = tree(&[
            "shop/artisan",
            "shop/composer.json",
            "api/Cargo.toml",
            "ml/pyproject.toml",
            "web/package.json",
            "notes/readme.txt",
            "Empty Folder/.keep",
        ]);
        fs::create_dir(dir.path().join("photos")).unwrap();
        let found = discover(&Catalog::bundled(), dir.path(), 1, &[]);
        assert_eq!(rels(&found), ["api", "ml", "shop", "web"]);
        let shop = found.projects.iter().find(|p| p.rel == "shop").unwrap();
        assert_eq!(shop.kind, ProjectKind::Laravel);
        assert_eq!(shop.name, "shop");
        assert!(found.errors.is_empty());
    }

    #[test]
    fn skips_dependencies_hidden_and_excluded_folders() {
        let dir = tree(&[
            "node_modules/pkg/package.json",
            "vendor/lib/composer.json",
            ".cache/app/package.json",
            "archive-2023/old/package.json",
            "keep/package.json",
            "clients/acme/package.json",
        ]);
        let exclude = ["archive-*".to_string(), "clients/acme".to_string()];
        let found = discover(&Catalog::bundled(), dir.path(), 2, &exclude);
        assert_eq!(rels(&found), ["keep"]);
    }

    #[test]
    fn goes_deeper_only_through_folders_that_are_not_projects() {
        let dir = tree(&[
            "clients/acme/package.json",
            "clients/beta/composer.json",
            "clients/deep/inner/most/package.json",
            "shop/package.json",
            "shop/packages/ui/package.json",
        ]);
        let shallow = discover(&Catalog::bundled(), dir.path(), 1, &[]);
        assert_eq!(rels(&shallow), ["shop"]);

        let deeper = discover(&Catalog::bundled(), dir.path(), 2, &[]);
        assert_eq!(
            rels(&deeper),
            ["clients/acme", "clients/beta", "shop"],
            "a project's own subfolders are never searched"
        );
        assert!(deeper.searched.contains(&dir.path().join("clients")));
        assert!(!deeper.searched.contains(&dir.path().join("shop")));

        let deepest = discover(&Catalog::bundled(), dir.path(), 3, &[]);
        assert!(!rels(&deepest).contains(&"clients/deep/inner/most"));
    }

    #[cfg(unix)]
    #[test]
    fn symlink_loops_and_duplicates_are_followed_once() {
        let dir = tree(&["shop/package.json"]);
        let root = dir.path();
        std::os::unix::fs::symlink(root, root.join("loop")).unwrap();
        std::os::unix::fs::symlink(root.join("shop"), root.join("shop-link")).unwrap();
        let found = discover(&Catalog::bundled(), root, 3, &[]);
        assert_eq!(found.projects.len(), 1, "{:?}", rels(&found));
    }

    #[test]
    fn found_paths_match_registered_paths_for_both_root_forms() {
        let dir = tree(&["shop/package.json"]);
        let expected = locdev_core::paths::plain(dir.path().join("shop").canonicalize().unwrap());
        for root in [dir.path().to_path_buf(), dir.path().canonicalize().unwrap()] {
            let result = discover(&Catalog::bundled(), &root, 1, &[]);
            assert_eq!(result.projects.len(), 1);
            assert_eq!(result.projects[0].path, expected);
            assert_eq!(result.projects[0].rel, "shop");
        }
    }

    #[test]
    fn a_folder_that_is_a_project_is_recognized() {
        let dir = tree(&["artisan", "composer.json"]);
        assert!(is_project(&Catalog::bundled(), dir.path()).is_some());
        let plain = tree(&["notes.txt"]);
        assert!(is_project(&Catalog::bundled(), plain.path()).is_none());
    }
}
