use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

fn main() {
    // Toolchain and build metadata surfaced to the UI (Settings → About).
    let rustc_version = Command::new(std::env::var("RUSTC").unwrap_or_else(|_| "rustc".into()))
        .arg("--version")
        .output()
        .ok()
        .and_then(|out| String::from_utf8(out.stdout).ok())
        .map(|s| s.trim().to_string())
        .unwrap_or_else(|| "rustc (unknown)".into());
    println!("cargo:rustc-env=RUSTC_VERSION={rustc_version}");

    let build_date = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs().to_string())
        .unwrap_or_default();
    println!("cargo:rustc-env=BUILD_TIMESTAMP={build_date}");

    tauri_build::build()
}
