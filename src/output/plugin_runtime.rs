use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use crate::output::plugin_client::EncoderPluginClient;
use crate::output::plugin_protocol::{
    validate_manifest, MaterializationRequest, MaterializationResponse, PluginManifest,
    PluginRequest, PluginResponse, ProtocolError,
};
use crate::output::plugin_runtime_error::PluginRuntimeError;

#[derive(Debug, Clone)]
pub struct SubprocessEncoderPluginClient {
    executable: PathBuf,
}

impl SubprocessEncoderPluginClient {
    pub fn new(executable: impl Into<PathBuf>) -> Self {
        Self {
            executable: executable.into(),
        }
    }

    pub fn executable(&self) -> &Path {
        &self.executable
    }

    fn invoke(&self, request: &PluginRequest) -> Result<PluginResponse, PluginRuntimeError> {
        if !self.executable.exists() {
            return Err(PluginRuntimeError::ExecutableNotFound {
                path: self.executable.clone(),
            });
        }

        let mut child = None;
        let mut last_error: Option<std::io::Error> = None;

        for _ in 0..5 {
            match Command::new(&self.executable)
                .stdin(Stdio::piped())
                .stdout(Stdio::piped())
                .stderr(Stdio::piped())
                .spawn()
            {
                Ok(process) => {
                    child = Some(process);
                    break;
                }
                Err(error) if error.raw_os_error() == Some(26) => {
                    last_error = Some(error);
                    std::thread::sleep(std::time::Duration::from_millis(25));
                }
                Err(error) => {
                    return Err(PluginRuntimeError::SpawnFailed {
                        path: self.executable.clone(),
                        message: error.to_string(),
                    });
                }
            }
        }

        let mut child = child.ok_or_else(|| {
            let message = last_error
                .map(|error| error.to_string())
                .unwrap_or_else(|| "failed to spawn plugin process".to_string());

            PluginRuntimeError::SpawnFailed {
                path: self.executable.clone(),
                message,
            }
        })?;

        {
            let stdin = child.stdin.as_mut().ok_or_else(|| PluginRuntimeError::Io {
                path: self.executable.clone(),
                message: "failed to open stdin for plugin process".to_string(),
            })?;

            serde_json::to_writer(&mut *stdin, request).map_err(|error| {
                PluginRuntimeError::Io {
                    path: self.executable.clone(),
                    message: format!("failed to serialize request: {error}"),
                }
            })?;

            stdin.flush().map_err(|error| PluginRuntimeError::Io {
                path: self.executable.clone(),
                message: format!("failed to flush request to plugin stdin: {error}"),
            })?;
        }

        let output = child
            .wait_with_output()
            .map_err(|error| PluginRuntimeError::Io {
                path: self.executable.clone(),
                message: format!("failed waiting for plugin process: {error}"),
            })?;

        if !output.status.success() {
            return Err(PluginRuntimeError::NonZeroExit {
                path: self.executable.clone(),
                status: output.status.code().unwrap_or(-1),
                stderr: String::from_utf8_lossy(&output.stderr).trim().to_string(),
            });
        }

        let response: PluginResponse = serde_json::from_slice(&output.stdout).map_err(|error| {
            PluginRuntimeError::InvalidJson {
                path: self.executable.clone(),
                message: error.to_string(),
            }
        })?;

        Ok(response)
    }

    pub fn manifest_runtime(&self) -> Result<PluginManifest, PluginRuntimeError> {
        match self.invoke(&PluginRequest::GetManifest)? {
            PluginResponse::Manifest { manifest } => {
                validate_manifest(&manifest)?;
                Ok(manifest)
            }
            PluginResponse::Error { error } => Err(PluginRuntimeError::Protocol {
                error: ProtocolError::InternalPluginError {
                    message: format!("plugin error [{}]: {}", error.code, error.message),
                },
            }),
            other => Err(PluginRuntimeError::UnexpectedResponse {
                path: self.executable.clone(),
                message: format!("expected manifest response, got {other:?}"),
            }),
        }
    }

    pub fn materialize_runtime(
        &self,
        request: &MaterializationRequest,
    ) -> Result<MaterializationResponse, PluginRuntimeError> {
        match self.invoke(&PluginRequest::Materialize {
            request: Box::new(request.clone()),
        })? {
            PluginResponse::Materialized { response } => Ok(response),
            PluginResponse::Error { error } => Err(PluginRuntimeError::Protocol {
                error: ProtocolError::MaterializationFailed {
                    message: format!("plugin error [{}]: {}", error.code, error.message),
                },
            }),
            other => Err(PluginRuntimeError::UnexpectedResponse {
                path: self.executable.clone(),
                message: format!("expected materialized response, got {other:?}"),
            }),
        }
    }
}

impl EncoderPluginClient for SubprocessEncoderPluginClient {
    fn manifest(&self) -> Result<PluginManifest, ProtocolError> {
        self.manifest_runtime()
            .map_err(|error| error.to_protocol_error())
    }

    fn materialize(
        &self,
        request: &MaterializationRequest,
    ) -> Result<MaterializationResponse, ProtocolError> {
        self.materialize_runtime(request)
            .map_err(|error| error.to_protocol_error())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::output::plugin_protocol::ProtocolInlineFile;
    use crate::output::plugin_protocol::{
        PluginManifest, ProtocolContentType, ProtocolEncoderCapability, ProtocolFormat,
        ENCODER_PLUGIN_PROTOCOL_V1,
    };
    use std::path::Path;
    use tempfile::TempDir;

    #[cfg(unix)]
    fn example_manifest() -> PluginManifest {
        PluginManifest {
            plugin_id: "plugin.subprocess".to_string(),
            plugin_version: "0.1.0".to_string(),
            protocol_version: ENCODER_PLUGIN_PROTOCOL_V1,
            display_name: Some("Subprocess Plugin".to_string()),
            description: Some("Fixture subprocess encoder plugin".to_string()),
            capabilities: vec![ProtocolEncoderCapability::new(
                "disc.iso",
                ProtocolContentType::Disc,
            )
            .supports_format(ProtocolFormat::Iso)],
        }
    }

    #[cfg(unix)]
    fn write_executable_script(dir: &TempDir, name: &str, body: &str) -> PathBuf {
        use std::fs::{self, File};
        use std::io::Write;
        use std::os::unix::fs::PermissionsExt;

        let tmp_path = dir.path().join(format!("{name}.tmp"));
        let final_path = dir.path().join(name);

        {
            let mut file = File::create(&tmp_path).unwrap();
            file.write_all(body.as_bytes()).unwrap();
            file.sync_all().unwrap();
        }

        let mut permissions = fs::metadata(&tmp_path).unwrap().permissions();
        permissions.set_mode(0o755);
        fs::set_permissions(&tmp_path, permissions).unwrap();

        fs::rename(&tmp_path, &final_path).unwrap();

        final_path
    }

    #[cfg(unix)]
    #[test]
    fn subprocess_client_reads_valid_manifest() {
        let temp_dir = TempDir::new().unwrap();

        let manifest_json = serde_json::to_string(&PluginResponse::Manifest {
            manifest: example_manifest(),
        })
        .unwrap();

        let script = format!(
            r#"#!/bin/sh
cat >/dev/null
printf '%s' '{}'
"#,
            manifest_json.replace('\'', "'\\''")
        );

        let path = write_executable_script(&temp_dir, "plugin-manifest.sh", &script);
        let client = SubprocessEncoderPluginClient::new(path);

        let manifest = client.manifest_runtime().unwrap();

        assert_eq!(manifest.plugin_id, "plugin.subprocess");
        assert_eq!(manifest.plugin_version, "0.1.0");
        assert_eq!(manifest.capabilities.len(), 1);
        assert_eq!(manifest.capabilities[0].capability_id, "disc.iso");
    }

    #[cfg(unix)]
    #[test]
    fn subprocess_client_rejects_unexpected_response_type() {
        let temp_dir = TempDir::new().unwrap();

        let response_json = serde_json::to_string(&PluginResponse::Materialized {
            response: MaterializationResponse::Inline(ProtocolInlineFile {
                bytes: vec![1, 2, 3],
            }),
        })
        .unwrap();

        let script = format!(
            "#!/bin/sh\ncat >/dev/null\nprintf '%s' '{}'\n",
            response_json.replace('\'', "'\\''")
        );

        let path = write_executable_script(&temp_dir, "plugin-wrong-response.sh", &script);
        let client = SubprocessEncoderPluginClient::new(path);

        let error = client.manifest_runtime().unwrap_err();

        match error {
            PluginRuntimeError::UnexpectedResponse { .. } => {}
            other => panic!("expected UnexpectedResponse, got {other:?}"),
        }
    }

    #[cfg(unix)]
    #[test]
    fn subprocess_client_surfaces_non_zero_exit() {
        let temp_dir = TempDir::new().unwrap();

        let script = r#"#!/bin/sh
cat >/dev/null
echo "boom" >&2
exit 12
"#;

        let path = write_executable_script(&temp_dir, "plugin-fails.sh", script);
        let client = SubprocessEncoderPluginClient::new(path);

        let error = client.manifest_runtime().unwrap_err();

        match error {
            PluginRuntimeError::NonZeroExit { status, stderr, .. } => {
                assert_eq!(status, 12);
                assert_eq!(stderr, "boom");
            }
            other => panic!("expected NonZeroExit, got {other:?}"),
        }
    }

    #[cfg(unix)]
    #[test]
    fn subprocess_client_surfaces_invalid_json() {
        let temp_dir = TempDir::new().unwrap();

        let script = r#"#!/bin/sh
cat >/dev/null
printf '%s' 'not-json'
"#;

        let path = write_executable_script(&temp_dir, "plugin-invalid-json.sh", script);
        let client = SubprocessEncoderPluginClient::new(path);

        let error = client.manifest_runtime().unwrap_err();

        match error {
            PluginRuntimeError::InvalidJson { .. } => {}
            other => panic!("expected InvalidJson, got {other:?}"),
        }
    }

    #[test]
    fn executable_path_is_exposed() {
        let path = Path::new("/tmp/example-plugin");
        let client = SubprocessEncoderPluginClient::new(path);

        assert_eq!(client.executable(), path);
    }
}
