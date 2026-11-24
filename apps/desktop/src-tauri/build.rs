//! Besides Tauri's own build step, records which commit and when this app
//! was built, for Settings → About. CI passes GITHUB_SHA; a local build asks
//! git. Both are optional: About shows "unknown" without them.

use std::process::Command;

fn main() {
    let commit = std::env::var("GITHUB_SHA").ok().or_else(|| {
        let out = Command::new("git")
            .args(["rev-parse", "HEAD"])
            .output()
            .ok()?;
        out.status
            .success()
            .then(|| String::from_utf8_lossy(&out.stdout).trim().to_string())
    });
    if let Some(commit) =
        commit.filter(|c| c.len() >= 7 && c.chars().all(|ch| ch.is_ascii_hexdigit()))
    {
        println!(
            "cargo:rustc-env=LOCDEV_COMMIT={}",
            &commit[..12.min(commit.len())]
        );
    }
    // Reproducible builds set SOURCE_DATE_EPOCH; otherwise, now.
    let seconds = std::env::var("SOURCE_DATE_EPOCH")
        .ok()
        .and_then(|s| s.parse::<u64>().ok())
        .unwrap_or_else(|| {
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_secs())
                .unwrap_or(0)
        });
    println!("cargo:rustc-env=LOCDEV_BUILD_TIME={seconds}");
    println!("cargo:rerun-if-env-changed=GITHUB_SHA");
    println!("cargo:rerun-if-env-changed=SOURCE_DATE_EPOCH");
    println!("cargo:rerun-if-changed=../../../.git/HEAD");
    println!("cargo:rerun-if-changed=../../../.git/logs/HEAD");
    tauri_build::build()
}
