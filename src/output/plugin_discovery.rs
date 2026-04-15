use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use crate::output::plugin_client::EncoderPluginClient;
use crate::output::plugin_protocol::{PluginManifest, ProtocolError};
use crate::output::plugin_registry::PluginRegistry;
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
                Ok(metadata) => metadata,
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

pub fn build_registry_from_discovery(
    report: PluginDiscoveryReport,
) -> Result<PluginRegistry, ProtocolError> {
    let mut registry = PluginRegistry::new();

    for plugin in report.discovered {
        registry.register(plugin.client)?;
    }

    Ok(registry)
}

#[cfg(test)]
mod tests {
    use std::fs;

    use tempfile::TempDir;

    use super::*;
    use crate::output::plugin_protocol::{
        PluginResponse, ProtocolContentType, ProtocolEncoderCapability, ProtocolFormat,
        ENCODER_PLUGIN_PROTOCOL_V1,
    };

    #[cfg(unix)]
    fn example_manifest() -> PluginManifest {
        PluginManifest {
            plugin_id: "plugin.discovery".to_string(),
            plugin_version: "1.0.0".to_string(),
            protocol_version: ENCODER_PLUGIN_PROTOCOL_V1,
            display_name: Some("Discovery Plugin".to_string()),
            description: Some("Fixture plugin for discovery tests".to_string()),
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
    fn discovers_valid_plugin() {
        let dir = TempDir::new().unwrap();

        let manifest_json = serde_json::to_string(&PluginResponse::Manifest {
            manifest: example_manifest(),
        })
        .unwrap();

        write_executable_script(
            &dir,
            "plugin-ok.sh",
            &format!(
                r#"#!/bin/sh
cat >/dev/null
cat <<'EOF'
{}
EOF
"#,
                manifest_json
            ),
        );

        let report = discover_encoder_plugins(dir.path());

        assert_eq!(report.discovered.len(), 1, "report: {report:?}");
        assert_eq!(report.rejected.len(), 0, "report: {report:?}");
        assert_eq!(report.discovered[0].manifest.plugin_id, "plugin.discovery");
    }

    #[cfg(unix)]
    #[test]
    fn rejects_invalid_plugin() {
        let dir = TempDir::new().unwrap();

        write_executable_script(
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

    #[cfg(unix)]
    #[test]
    fn skips_non_executable_files() {
        let dir = TempDir::new().unwrap();

        let path = dir.path().join("not-executable.sh");
        fs::write(&path, "#!/bin/sh\nexit 0\n").unwrap();

        let report = discover_encoder_plugins(dir.path());

        assert_eq!(report.discovered.len(), 0);
        assert_eq!(report.rejected.len(), 0);
    }

    #[cfg(unix)]
    #[test]
    fn builds_registry_from_discovered_plugins() {
        let dir = TempDir::new().unwrap();

        let manifest_json = serde_json::to_string(&PluginResponse::Manifest {
            manifest: example_manifest(),
        })
        .unwrap();

        write_executable_script(
            &dir,
            "plugin-ok.sh",
            &format!(
                r#"#!/bin/sh
cat >/dev/null
cat <<'EOF'
{}
EOF
"#,
                manifest_json
            ),
        );

        let report = discover_encoder_plugins(dir.path());
        let registry = build_registry_from_discovery(report).unwrap();
        let encoders = registry.build_encoders().unwrap();

        assert_eq!(encoders.len(), 1);
        assert_eq!(encoders[0].plugin_id(), "plugin.discovery");
    }

    #[cfg(unix)]
    #[test]
    fn discovered_plugin_can_materialize_plan() {
        use crate::core::source::SourceRef;
        use crate::core::vfs::VfsNode;
        use crate::output::capabilities::{CapabilityRequirements, ContentType, Format};
        use crate::output::materialize::materialize_plan_with_plugins;
        use crate::output::plan::{
            ArtifactId, ArtifactRequest, PlanEntry, PlanFile, PlannedArtifactKind,
            PresentationPlan, SourceArtifact,
        };
        use crate::output::plugin_protocol::{
            MaterializationResponse, PluginResponse, ProtocolInlineFile,
        };

        let dir = TempDir::new().unwrap();

        let manifest_json = serde_json::to_string(&PluginResponse::Manifest {
            manifest: example_manifest(),
        })
        .unwrap();

        let materialize_json = serde_json::to_string(&PluginResponse::Materialized {
            response: MaterializationResponse::Inline(ProtocolInlineFile {
                bytes: b"plugin-output".to_vec(),
            }),
        })
        .unwrap();

        let script = format!(
            r#"#!/bin/sh
request="$(cat)"
case "$request" in
  *'"type":"get_manifest"'*)
    cat <<'EOF'
{}
EOF
    ;;
  *)
    cat <<'EOF'
{}
EOF
    ;;
esac
"#,
            manifest_json, materialize_json
        );

        write_executable_script(&dir, "plugin-ok.sh", &script);

        let report = discover_encoder_plugins(dir.path());
        assert_eq!(report.discovered.len(), 1, "report: {report:?}");
        assert_eq!(report.rejected.len(), 0, "report: {report:?}");

        let registry = build_registry_from_discovery(report).unwrap();

        let plan = PresentationPlan::new(vec![PlanEntry::File(PlanFile::new(
            "Game.iso",
            ArtifactRequest::new(
                ArtifactId::new("game"),
                PlannedArtifactKind::SourceBacked(SourceArtifact::single(
                    SourceRef::new("file:/roms/game.iso"),
                    4096,
                )),
                CapabilityRequirements::new(ContentType::Disc).with_format(Format::Iso),
            ),
        ))]);

        let root = materialize_plan_with_plugins(&plan, &registry).unwrap();

        assert_eq!(root.children().len(), 1);

        let file = match &root.children()[0] {
            VfsNode::File(file) => file,
            other => panic!("expected file, got {other:?}"),
        };

        match &file.backing {
            crate::core::vfs::FileBacking::Inline(contents) => {
                assert_eq!(contents, b"plugin-output");
                assert_eq!(file.size, b"plugin-output".len() as u64);
            }
            other => panic!("expected inline plugin output, got {other:?}"),
        }
    }
}
