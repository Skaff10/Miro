fn main() {
    tauri_build::build();

    #[cfg(windows)]
    emit_windows_test_manifest();
}

#[cfg(windows)]
fn emit_windows_test_manifest() {
    use std::path::PathBuf;
    use std::process::Command;

    let Ok(out_dir) = std::env::var("OUT_DIR") else {
        return;
    };
    let out = PathBuf::from(out_dir);
    let manifest_path = out.join("test.manifest");
    let rc_path = out.join("test_manifest.rc");
    let obj_path = out.join("test_manifest.o");

    let manifest_xml = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<assembly xmlns="urn:schemas-microsoft-com:asm.v1" manifestVersion="1.0">
  <dependency>
    <dependentAssembly>
      <assemblyIdentity
        type="win32"
        name="Microsoft.Windows.Common-Controls"
        version="6.0.0.0"
        processorArchitecture="*"
        publicKeyToken="6595b64144ccf1df"
        language="*"
      />
    </dependentAssembly>
  </dependency>
</assembly>
"#;

    if std::fs::write(&manifest_path, manifest_xml).is_err() {
        return;
    }
    let rc_content = format!(
        "1 24 \"{}\"\n",
        manifest_path.to_string_lossy().replace('\\', "/")
    );
    if std::fs::write(&rc_path, rc_content).is_err() {
        return;
    }

    let env = std::env::var("CARGO_CFG_TARGET_ENV").unwrap_or_default();
    if env == "gnu" {
        let status = Command::new("windres")
            .args([
                rc_path.to_string_lossy().as_ref(),
                "-O",
                "coff",
                "-o",
                obj_path.to_string_lossy().as_ref(),
            ])
            .status();
        if status.is_ok_and(|s| s.success()) {
            println!("cargo:rustc-link-arg-tests={}", obj_path.display());
        }
    }
}
