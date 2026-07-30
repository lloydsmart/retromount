use std::fs;
use std::process::Command;

#[test]
fn inspect_accepts_an_external_yaml_presentation() {
    let temp_dir = tempfile::tempdir().unwrap();
    let input_path = temp_dir.path().join("payload.dat");
    let presentation_path = temp_dir.path().join("bytes.yaml");

    fs::write(&input_path, b"external presentation bytes").unwrap();
    fs::write(
        &presentation_path,
        r#"
version: 1
name: external-bytes
layout:
  type: literal_root
  path: Files
files:
  - select:
      type: bytes
    naming:
      type: source_name
    artifact:
      content_type: bytes
      format: bin
"#,
    )
    .unwrap();

    let output = Command::new(env!("CARGO_BIN_EXE_retromount"))
        .arg("inspect")
        .arg(&input_path)
        .arg("--presentation")
        .arg(&presentation_path)
        .output()
        .unwrap();

    assert!(
        output.status.success(),
        "inspect failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains("Output VFS:"));
    assert!(stdout.contains("Files/"));
    assert!(
        stdout.contains("payload.dat"),
        "unexpected output:\n{stdout}"
    );
}

#[test]
fn inspect_reports_external_presentation_validation_errors() {
    let temp_dir = tempfile::tempdir().unwrap();
    let input_path = temp_dir.path().join("payload.dat");
    let presentation_path = temp_dir.path().join("invalid.yaml");

    fs::write(&input_path, b"bytes").unwrap();
    fs::write(
        &presentation_path,
        r#"
version: 99
name: invalid
layout:
  type: flat
files: []
"#,
    )
    .unwrap();

    let output = Command::new(env!("CARGO_BIN_EXE_retromount"))
        .arg("inspect")
        .arg(&input_path)
        .arg("--presentation")
        .arg(&presentation_path)
        .output()
        .unwrap();

    assert!(!output.status.success());
    let stderr = String::from_utf8(output.stderr).unwrap();
    assert!(stderr.contains("unsupported schema version 99"));
    assert!(stderr.contains(presentation_path.to_str().unwrap()));
}
