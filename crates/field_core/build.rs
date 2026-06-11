use std::{
    env,
    process::Command,
    time::{SystemTime, UNIX_EPOCH},
};

fn main() {
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-env-changed=RUSTC");
    println!("cargo:rerun-if-env-changed=PROFILE");

    let rustc = env::var("RUSTC").expect("missing required env var RUSTC");
    if let Ok(out) = Command::new(rustc).arg("--version").output() {
        if out.status.success() {
            if let Ok(ver) = String::from_utf8(out.stdout) {
                println!("cargo:rustc-env=RUSTC_VERSION={}", ver.trim());
            }
        }
    }

    if let Ok(out) = Command::new("git")
        .args(["rev-parse", "--short=12", "HEAD"])
        .output()
    {
        if out.status.success() {
            let mut sha = String::from_utf8_lossy(&out.stdout).trim().to_string();
            let dirty = Command::new("git")
                .args(["status", "--porcelain"])
                .output()
                .ok()
                .map_or(false, |o| o.status.success() && !o.stdout.is_empty());
            if dirty {
                sha.push_str("-dirty");
            }
            println!("cargo:rustc-env=GIT_HASH={sha}");
        }
    }

    let profile = env::var("PROFILE").expect("missing required env var PROFILE");
    println!("cargo:rustc-env=PROFILE={profile}");

    let epoch = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .expect("system time before unix epoch");
    println!("cargo:rustc-env=BUILD_TIME_UNIX={epoch}");
}
