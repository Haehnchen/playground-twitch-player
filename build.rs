use std::process::Command;

fn main() {
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-env-changed=PKG_CONFIG_PATH");
    println!("cargo:rerun-if-env-changed=PKG_CONFIG_LIBDIR");
    println!("cargo:rerun-if-env-changed=PKG_CONFIG_SYSROOT_DIR");

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
