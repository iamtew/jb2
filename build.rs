use std::process::Command;

fn main() {
    println!("cargo:rerun-if-changed=.git/HEAD");
    println!("cargo:rerun-if-changed=.git/index");
    println!(
        "cargo:rustc-env=JB2_VERSION={}",
        build_version(env!("CARGO_PKG_VERSION"))
    );

    #[cfg(windows)]
    {
        let ico = std::path::Path::new("webroot/img/jukeboks.ico");
        if ico.exists() {
            let mut res = winres::WindowsResource::new();
            res.set_icon("webroot/img/jukeboks.ico");
            let _ = res.compile();
        }
    }
}

fn build_version(pkg: &str) -> String {
    let tag = git_stdout(&["describe", "--tags", "--abbrev=0"]).unwrap_or_else(|| pkg.to_string());
    let hash = git_stdout(&["rev-parse", "--short", "HEAD"]).unwrap_or_else(|| "unknown".into());
    let dirty = git_stdout(&["status", "--porcelain"]).is_some_and(|s| !s.is_empty());
    if dirty {
        format!("{tag}-{hash}-dirty")
    } else {
        format!("{tag}-{hash}")
    }
}

fn git_stdout(args: &[&str]) -> Option<String> {
    let out = Command::new("git").args(args).output().ok()?;
    if !out.status.success() {
        return None;
    }
    let s = String::from_utf8_lossy(&out.stdout).trim().to_string();
    if s.is_empty() {
        None
    } else {
        Some(s)
    }
}
