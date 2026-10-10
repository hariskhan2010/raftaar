fn main() {
    // Release builds always run as administrator (Windows shows UAC on launch).
    // Dev builds don't, so `tauri dev` works from a normal terminal; the app
    // shows a banner and HKLM tweaks fail until you run it elevated.
    let attrs = if std::env::var("PROFILE").as_deref() == Ok("release") {
        tauri_build::WindowsAttributes::new().app_manifest(include_str!("admin.manifest"))
    } else {
        tauri_build::WindowsAttributes::new()
    };
    tauri_build::try_build(tauri_build::Attributes::new().windows_attributes(attrs)).expect("tauri build failed");
}
