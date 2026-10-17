fn main() {
    let mut attributes = tauri_build::Attributes::new();

    // tauri-build embeds its Windows app manifest (which pulls in Common Controls v6,
    // needed by the dialog APIs) into bin targets only. Test executables then load
    // comctl32 v5 and fail to start with STATUS_ENTRYPOINT_NOT_FOUND. Have the linker
    // embed the same manifest into every executable instead.
    let target_env = std::env::var("CARGO_CFG_TARGET_ENV").unwrap_or_default();
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("windows") && target_env == "msvc" {
        let manifest = std::path::Path::new(&std::env::var("CARGO_MANIFEST_DIR").unwrap())
            .join("windows-app-manifest.xml");
        println!("cargo:rerun-if-changed={}", manifest.display());
        println!("cargo:rustc-link-arg=/MANIFEST:EMBED");
        println!("cargo:rustc-link-arg=/MANIFESTINPUT:{}", manifest.display());
        attributes = attributes
            .windows_attributes(tauri_build::WindowsAttributes::new_without_app_manifest());
    }

    tauri_build::try_build(attributes).expect("failed to run tauri-build");
}
