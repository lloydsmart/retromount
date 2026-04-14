use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use crate::output::plugin_client::EncoderPluginClient;
use crate::output::plugin_protocol::PluginManifest;
use crate::output::plugin_runtime::SubprocessEncoderPluginClient;
use crate::output::plugin_runtime_error::PluginRuntimeError;

#[derive(Clone)]
pub struct DiscoveredPlugin {
    pub executable: PathBuf,
    pub manifest: PluginManifest,
    pub client: Arc<dyn EncoderPluginClient>,
}

impl std::fmt::Debug for DiscoveredPlugin {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("DiscoveredPlugin")
            .field("executable", &self.executable)
            .field("manifest", &self.manifest)
            .finish()
    }
}

#[derive(Debug)]
pub struct RejectedPlugin {
    pub executable: PathBuf,
    pub error: PluginRuntimeError,
}

#[derive(Debug)]
pub struct PluginDiscoveryReport {
    pub discovered: Vec<DiscoveredPlugin>,
    pub rejected: Vec<RejectedPlugin>,
}

pub fn discover_encoder_plugins(dir: &Path) -> PluginDiscoveryReport {
    let mut discovered = Vec::new();
    let mut rejected = Vec::new();

    let entries = match fs::read_dir(dir) {
        Ok(entries) => entries,
        Err(error) => {
            rejected.push(RejectedPlugin {
                executable: dir.to_path_buf(),
                error: PluginRuntimeError::Io {
                    path: dir.to_path_buf(),
                    message: error.to_string(),
                },
            });
            return PluginDiscoveryReport {
                discovered,
                rejected,
            };
        }
    };

    for entry in entries.flatten() {
        let path = entry.path();

        if !path.is_file() {
            continue;
        }

        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;

            let metadata = match entry.metadata() {
                Ok(m) => m,
                Err(_) => continue,
            };

            if metadata.permissions().mode() & 0o111 == 0 {
                continue;
            }
        }

        let client = SubprocessEncoderPluginClient::new(&path);

        match client.manifest_runtime() {
            Ok(manifest) => {
                discovered.push(DiscoveredPlugin {
                    executable: path.clone(),
                    manifest,
                    client: Arc::new(client),
                });
            }
            Err(error) => {
                rejected.push(RejectedPlugin {
                    executable: path.clone(),
                    error,
                });
            }
        }
    }

    PluginDiscoveryReport {
        discovered,
        rejected,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::output::plugin_protocol::{
        PluginManifest, PluginResponse, ProtocolContentType, ProtocolEncoderCapability,
        ProtocolFormat, ENCODER_PLUGIN_PROTOCOL_V1,
    };

    use tempfile::TempDir;

    #[cfg(unix)]
    use std::os::unix::fs::PermissionsExt;

    use std::fs;

    fn example_manifest() -> PluginManifest {
        PluginManifest {
            plugin_id: "plugin.discovery".to_string(),
            plugin_version: "1.0.0".to_string(),
            protocol_version: ENCODER_PLUGIN_PROTOCOL_V1,
            display_name: None,
            description: None,
            capabilities: vec![ProtocolEncoderCapability::new(
                "disc.iso",
                ProtocolContentType::Disc,
            )
            .supports_format(ProtocolFormat::Iso)],
        }
    }

    #[cfg(unix)]
    fn write_exec(dir: &TempDir, name: &str, body: &str) -> PathBuf {
        let path = dir.path().join(name);
        fs::write(&path, body).unwrap();

        let mut perms = fs::metadata(&path).unwrap().permissions();
        perms.set_mode(0o755);
        fs::set_permissions(&path, perms).unwrap();

        path
    }

    #[cfg(unix)]
    #[test]
    fn discovers_valid_plugin() {
        let dir = TempDir::new().unwrap();

        let manifest_json = serde_json::to_string(&PluginResponse::Manifest {
            manifest: example_manifest(),
        })
        .unwrap();

        write_exec(
            &dir,
            "plugin-ok.sh",
            &format!(
                r#"#!/bin/sh
cat >/dev/null
printf '%s' '{}'
"#,
                manifest_json.replace('\'', "'\\''")
            ),
        );

        let report = discover_encoder_plugins(dir.path());

        assert_eq!(report.discovered.len(), 1);
        assert_eq!(report.rejected.len(), 0);

        assert_eq!(report.discovered[0].manifest.plugin_id, "plugin.discovery");
    }

    #[cfg(unix)]
    #[test]
    fn rejects_invalid_plugin() {
        let dir = TempDir::new().unwrap();

        write_exec(
            &dir,
            "plugin-bad.sh",
            r#"#!/bin/sh
cat >/dev/null
printf '%s' 'not-json'
"#,
        );

        let report = discover_encoder_plugins(dir.path());

        assert_eq!(report.discovered.len(), 0);
        assert_eq!(report.rejected.len(), 1);
    }
}
