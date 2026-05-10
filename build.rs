use std::{env, process::Command};

const DEFAULT_BUILD_VERSION: &str = "nightly";
const DEFAULT_BUILD_DATE: &str = "local";
const APP_AUTHOR: &str = "Daniel Espendiller";

fn main() {
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-env-changed=BUILD_VERSION");
    println!("cargo:rerun-if-env-changed=BUILD_DATE");
    println!("cargo:rerun-if-env-changed=PKG_CONFIG_PATH");
    println!("cargo:rerun-if-env-changed=PKG_CONFIG_LIBDIR");
    println!("cargo:rerun-if-env-changed=PKG_CONFIG_SYSROOT_DIR");
    println!(
        "cargo:rustc-env=TWITCH_PLAYER_BUILD_VERSION={}",
        env::var("BUILD_VERSION").unwrap_or_else(|_| DEFAULT_BUILD_VERSION.to_string())
    );
    println!(
        "cargo:rustc-env=TWITCH_PLAYER_BUILD_DATE={}",
        env::var("BUILD_DATE").unwrap_or_else(|_| build_date())
    );
    println!("cargo:rustc-env=TWITCH_PLAYER_AUTHOR={APP_AUTHOR}");

    let packages = [
        "gtk4",
        "gio-2.0",
        "gdk-pixbuf-2.0",
        "mpv",
        "epoxy",
        "json-glib-1.0",
        "libsoup-3.0",
    ];

    let output = Command::new("pkg-config")
        .arg("--libs")
        .arg("--print-errors")
        .args(packages)
        .output()
        .expect("pkg-config is required to find system libraries");

    if !output.status.success() {
        panic!(
            "pkg-config failed:\n{}",
            String::from_utf8_lossy(&output.stderr)
        );
    }

    let libs = String::from_utf8(output.stdout).expect("pkg-config output must be UTF-8");
    for token in libs.split_whitespace() {
        if let Some(path) = token.strip_prefix("-L") {
            println!("cargo:rustc-link-search=native={path}");
        } else if let Some(lib) = token.strip_prefix("-l") {
            println!("cargo:rustc-link-lib={lib}");
        } else if token == "-pthread" || token.starts_with("-Wl,") {
            println!("cargo:rustc-link-arg={token}");
        }
    }
}

fn build_date() -> String {
    Command::new("date")
        .arg("-u")
        .arg("+%Y-%m-%d")
        .output()
        .ok()
        .filter(|output| output.status.success())
        .and_then(|output| String::from_utf8(output.stdout).ok())
        .map(|date| date.trim().to_string())
        .filter(|date| !date.is_empty())
        .unwrap_or_else(|| DEFAULT_BUILD_DATE.to_string())
}
