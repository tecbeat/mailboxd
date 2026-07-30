use std::{io::Result, process::Command};

fn main() -> Result<()> {
    if cfg!(target_os = "windows") {
        println!("cargo:rustc-link-lib=Rstrtmgr");
    }

    // Version: the teccave pipeline injects VERSION as a docker build-arg;
    // fall back to the cargo package version for local builds.
    let version = std::env::var("VERSION")
        .unwrap_or_else(|_| env!("CARGO_PKG_VERSION").to_string());
    println!("cargo:rustc-env=MAILBOXD_VERSION={}", version);
    println!("cargo:rerun-if-env-changed=VERSION");

    // Short git hash for the boot banner. Optional: absence must not fail
    // the build inside containers where .git may be missing.
    let git_hash = Command::new("git")
        .args(["rev-parse", "--short", "HEAD"])
        .output()
        .ok()
        .and_then(|o| {
            if o.status.success() {
                String::from_utf8(o.stdout).ok().map(|s| s.trim().to_string())
            } else {
                None
            }
        })
        .unwrap_or_else(|| "unknown".to_string());
    println!("cargo:rustc-env=GIT_HASH={}", git_hash);
    Ok(())
}
