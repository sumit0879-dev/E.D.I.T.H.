fn main() {
    tauri_build::build();

    #[cfg(target_os = "windows")]
    {
        // On Windows MSVC, test harness executables (e.g. `cargo test --lib`) require
        // Common-Controls v6 in their manifest to dynamically resolve `TaskDialogIndirect`
        // imported by `tauri-plugin-dialog` from `comctl32.dll`. Without this manifest,
        // Windows links comctl32.dll v5.82 from System32 which lacks TaskDialogIndirect,
        // causing the test binary to fail startup with STATUS_ENTRYPOINT_NOT_FOUND (0xc0000139).
        let target_env = std::env::var("CARGO_CFG_TARGET_ENV").unwrap_or_default();
        if target_env == "msvc" || target_env.is_empty() {
            println!("cargo:rustc-link-arg=comctl32.lib");
            println!("cargo:rustc-link-arg=/MANIFEST:EMBED");
            println!(
                "cargo:rustc-link-arg=/MANIFESTDEPENDENCY:type='win32' name='Microsoft.Windows.Common-Controls' version='6.0.0.0' processorArchitecture='*' publicKeyToken='6595b64144ccf1df' language='*'"
            );
        }
    }
}
