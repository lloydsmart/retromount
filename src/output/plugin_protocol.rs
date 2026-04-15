use std::collections::BTreeSet;

use serde::{Deserialize, Serialize};

pub const ENCODER_PLUGIN_PROTOCOL_V1: ProtocolVersion = ProtocolVersion::new(1, 0);

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct ProtocolVersion {
    pub major: u16,
    pub minor: u16,
}

impl ProtocolVersion {
    pub const fn new(major: u16, minor: u16) -> Self {
        Self { major, minor }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PluginManifest {
    pub plugin_id: String,
    pub plugin_version: String,
    pub protocol_version: ProtocolVersion,
    pub display_name: Option<String>,
    pub description: Option<String>,
    pub capabilities: Vec<ProtocolEncoderCapability>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProtocolEncoderCapability {
    pub capability_id: String,
    pub content_type: ProtocolContentType,
    pub formats: BTreeSet<ProtocolFormat>,
    pub features: BTreeSet<ProtocolCapabilityFeature>,
    pub priority: u32,
}

impl ProtocolEncoderCapability {
    pub fn new(capability_id: impl Into<String>, content_type: ProtocolContentType) -> Self {
        Self {
            capability_id: capability_id.into(),
            content_type,
            formats: BTreeSet::new(),
            features: BTreeSet::new(),
            priority: 0,
        }
    }

    pub fn supports_format(mut self, format: ProtocolFormat) -> Self {
        self.formats.insert(format);
        self
    }

    pub fn with_feature(mut self, feature: ProtocolCapabilityFeature) -> Self {
        self.features.insert(feature);
        self
    }

    pub fn with_priority(mut self, priority: u32) -> Self {
        self.priority = priority;
        self
    }

    pub fn supports_multi_source(&self) -> bool {
        self.features
            .contains(&ProtocolCapabilityFeature::MultiSource)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum ProtocolContentType {
    Rom,
    Disc,
    Playlist,
    Archive,
    Directory,
    Bytes,
    Text,
    Game,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum ProtocolFormat {
    Iso,
    Chd,
    Zip,
    M3u,
    Directory,
    Bin,
    Text,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum ProtocolCapabilityFeature {
    MultiSource,
    Streaming,
    Lossless,
    RandomAccess,
    SupportsPartial,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProtocolCapabilityRequirements {
    pub content_type: ProtocolContentType,
    pub format: Option<ProtocolFormat>,
    pub required_features: BTreeSet<ProtocolCapabilityFeature>,
    pub preferred_features: BTreeSet<ProtocolCapabilityFeature>,
    pub forbidden_features: BTreeSet<ProtocolCapabilityFeature>,
}

impl ProtocolCapabilityRequirements {
    pub fn new(content_type: ProtocolContentType) -> Self {
        Self {
            content_type,
            format: None,
            required_features: BTreeSet::new(),
            preferred_features: BTreeSet::new(),
            forbidden_features: BTreeSet::new(),
        }
    }

    pub fn with_format(mut self, format: ProtocolFormat) -> Self {
        self.format = Some(format);
        self
    }

    pub fn require_feature(mut self, feature: ProtocolCapabilityFeature) -> Self {
        self.required_features.insert(feature);
        self
    }

    pub fn prefer_feature(mut self, feature: ProtocolCapabilityFeature) -> Self {
        self.preferred_features.insert(feature);
        self
    }

    pub fn forbid_feature(mut self, feature: ProtocolCapabilityFeature) -> Self {
        self.forbidden_features.insert(feature);
        self
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MaterializationRequest {
    pub artifact_id: String,
    pub logical_name: String,
    pub selected_capability_id: String,
    pub artifact_kind: ProtocolArtifactKind,
    pub requirements: ProtocolCapabilityRequirements,
    pub context: ProtocolMaterializationContext,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ProtocolArtifactKind {
    SourceBacked(ProtocolSourceArtifact),
    Generated(ProtocolGeneratedArtifact),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProtocolSourceArtifact {
    pub inputs: Vec<ProtocolSourceArtifactInput>,
}

impl ProtocolSourceArtifact {
    pub fn single(source: impl Into<String>, size: u64) -> Self {
        Self {
            inputs: vec![ProtocolSourceArtifactInput {
                source: source.into(),
                size,
            }],
        }
    }

    pub fn multiple(inputs: Vec<ProtocolSourceArtifactInput>) -> Self {
        Self { inputs }
    }

    pub fn is_multi_source(&self) -> bool {
        self.inputs.len() > 1
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProtocolSourceArtifactInput {
    pub source: String,
    pub size: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ProtocolGeneratedArtifact {
    Playlist(ProtocolPlaylistArtifact),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProtocolPlaylistArtifact {
    pub entries: Vec<ProtocolArtifactReference>,
}

impl ProtocolPlaylistArtifact {
    pub fn new(entries: Vec<ProtocolArtifactReference>) -> Self {
        Self { entries }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProtocolArtifactReference {
    pub artifact_id: String,
}

impl ProtocolArtifactReference {
    pub fn new(artifact_id: impl Into<String>) -> Self {
        Self {
            artifact_id: artifact_id.into(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct ProtocolMaterializationContext {
    pub artifact_names: Vec<ProtocolArtifactName>,
}

impl ProtocolMaterializationContext {
    pub fn with_artifact_name(
        mut self,
        artifact_id: impl Into<String>,
        logical_name: impl Into<String>,
    ) -> Self {
        self.artifact_names.push(ProtocolArtifactName {
            artifact_id: artifact_id.into(),
            logical_name: logical_name.into(),
        });
        self
    }

    pub fn artifact_name_for(&self, artifact_id: &str) -> Option<&str> {
        self.artifact_names
            .iter()
            .find(|entry| entry.artifact_id == artifact_id)
            .map(|entry| entry.logical_name.as_str())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProtocolArtifactName {
    pub artifact_id: String,
    pub logical_name: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum PluginRequest {
    GetManifest,
    Materialize {
        request: Box<MaterializationRequest>,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum PluginResponse {
    Manifest { manifest: PluginManifest },
    Materialized { response: MaterializationResponse },
    Error { error: ProtocolPluginError },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProtocolPluginError {
    pub code: String,
    pub message: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum MaterializationResponse {
    SourceBacked(ProtocolMaterializedSourceFile),
    Inline(ProtocolInlineFile),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProtocolMaterializedSourceFile {
    pub source: String,
    pub size: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProtocolInlineFile {
    pub bytes: Vec<u8>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProtocolError {
    IncompatibleProtocolVersion {
        host: ProtocolVersion,
        plugin: ProtocolVersion,
    },
    InvalidManifest {
        message: String,
    },
    UnknownCapabilityId {
        capability_id: String,
    },
    InvalidRequest {
        message: String,
    },
    MissingContext {
        message: String,
    },
    UnsupportedArtifactKind {
        message: String,
    },
    MaterializationFailed {
        message: String,
    },
    InternalPluginError {
        message: String,
    },
}

pub fn ensure_protocol_compatible(
    host: &ProtocolVersion,
    plugin: &ProtocolVersion,
) -> Result<(), ProtocolError> {
    if host == plugin {
        Ok(())
    } else {
        Err(ProtocolError::IncompatibleProtocolVersion {
            host: *host,
            plugin: *plugin,
        })
    }
}

pub fn validate_manifest(manifest: &PluginManifest) -> Result<(), ProtocolError> {
    if manifest.plugin_id.trim().is_empty() {
        return Err(ProtocolError::InvalidManifest {
            message: "plugin_id must not be empty".to_string(),
        });
    }

    if manifest.plugin_version.trim().is_empty() {
        return Err(ProtocolError::InvalidManifest {
            message: "plugin_version must not be empty".to_string(),
        });
    }

    ensure_protocol_compatible(&ENCODER_PLUGIN_PROTOCOL_V1, &manifest.protocol_version)?;

    let mut capability_ids = BTreeSet::new();

    for capability in &manifest.capabilities {
        if capability.capability_id.trim().is_empty() {
            return Err(ProtocolError::InvalidManifest {
                message: "capability_id must not be empty".to_string(),
            });
        }

        if capability.formats.is_empty() {
            return Err(ProtocolError::InvalidManifest {
                message: format!(
                    "capability '{}' must declare at least one format",
                    capability.capability_id
                ),
            });
        }

        if !capability_ids.insert(capability.capability_id.clone()) {
            return Err(ProtocolError::InvalidManifest {
                message: format!("duplicate capability_id '{}'", capability.capability_id),
            });
        }
    }

    Ok(())
}

pub fn validate_materialization_request(
    request: &MaterializationRequest,
    manifest: &PluginManifest,
) -> Result<(), ProtocolError> {
    if request.artifact_id.trim().is_empty() {
        return Err(ProtocolError::InvalidRequest {
            message: "artifact_id must not be empty".to_string(),
        });
    }

    if request.logical_name.trim().is_empty() {
        return Err(ProtocolError::InvalidRequest {
            message: "logical_name must not be empty".to_string(),
        });
    }

    if request.selected_capability_id.trim().is_empty() {
        return Err(ProtocolError::InvalidRequest {
            message: "selected_capability_id must not be empty".to_string(),
        });
    }

    let capability = manifest
        .capabilities
        .iter()
        .find(|capability| capability.capability_id == request.selected_capability_id)
        .ok_or_else(|| ProtocolError::UnknownCapabilityId {
            capability_id: request.selected_capability_id.clone(),
        })?;

    if capability.content_type != request.requirements.content_type {
        return Err(ProtocolError::InvalidRequest {
            message: format!(
                "selected capability '{}' content type does not match request requirements",
                capability.capability_id
            ),
        });
    }

    if let Some(required_format) = request.requirements.format {
        if !capability.formats.contains(&required_format) {
            return Err(ProtocolError::InvalidRequest {
                message: format!(
                    "selected capability '{}' does not support required format '{required_format:?}'",
                    capability.capability_id
                ),
            });
        }
    }

    for feature in &request.requirements.required_features {
        if !capability.features.contains(feature) {
            return Err(ProtocolError::InvalidRequest {
                message: format!(
                    "selected capability '{}' is missing required feature '{feature:?}'",
                    capability.capability_id
                ),
            });
        }
    }

    for feature in &request.requirements.forbidden_features {
        if capability.features.contains(feature) {
            return Err(ProtocolError::InvalidRequest {
                message: format!(
                    "selected capability '{}' includes forbidden feature '{feature:?}'",
                    capability.capability_id
                ),
            });
        }
    }

    match &request.artifact_kind {
        ProtocolArtifactKind::SourceBacked(source_artifact) => {
            if source_artifact.inputs.is_empty() {
                return Err(ProtocolError::InvalidRequest {
                    message: "source-backed artifact must include at least one input".to_string(),
                });
            }

            if source_artifact.is_multi_source() && !capability.supports_multi_source() {
                return Err(ProtocolError::InvalidRequest {
                    message: format!(
                        "selected capability '{}' does not support multi-source artifacts",
                        capability.capability_id
                    ),
                });
            }
        }
        ProtocolArtifactKind::Generated(ProtocolGeneratedArtifact::Playlist(playlist)) => {
            if playlist.entries.is_empty() {
                return Err(ProtocolError::InvalidRequest {
                    message: "playlist artifact must include at least one entry".to_string(),
                });
            }

            for entry in &playlist.entries {
                if request
                    .context
                    .artifact_name_for(&entry.artifact_id)
                    .is_none()
                {
                    return Err(ProtocolError::MissingContext {
                        message: format!(
                            "missing artifact name for referenced artifact '{}'",
                            entry.artifact_id
                        ),
                    });
                }
            }
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn valid_manifest() -> PluginManifest {
        PluginManifest {
            plugin_id: "plugin.example".to_string(),
            plugin_version: "0.1.0".to_string(),
            protocol_version: ENCODER_PLUGIN_PROTOCOL_V1,
            display_name: Some("Example Plugin".to_string()),
            description: Some("Example encoder plugin".to_string()),
            capabilities: vec![
                ProtocolEncoderCapability::new("disc.chd", ProtocolContentType::Disc)
                    .supports_format(ProtocolFormat::Chd)
                    .with_feature(ProtocolCapabilityFeature::Lossless)
                    .with_priority(100),
                ProtocolEncoderCapability::new("playlist.m3u", ProtocolContentType::Playlist)
                    .supports_format(ProtocolFormat::M3u)
                    .with_feature(ProtocolCapabilityFeature::MultiSource),
            ],
        }
    }

    #[test]
    fn accepts_valid_manifest() {
        let manifest = valid_manifest();
        assert_eq!(validate_manifest(&manifest), Ok(()));
    }

    #[test]
    fn rejects_empty_plugin_id() {
        let mut manifest = valid_manifest();
        manifest.plugin_id = String::new();

        assert_eq!(
            validate_manifest(&manifest),
            Err(ProtocolError::InvalidManifest {
                message: "plugin_id must not be empty".to_string(),
            })
        );
    }

    #[test]
    fn rejects_duplicate_capability_ids() {
        let mut manifest = valid_manifest();
        manifest.capabilities.push(
            ProtocolEncoderCapability::new("disc.chd", ProtocolContentType::Disc)
                .supports_format(ProtocolFormat::Chd),
        );

        assert_eq!(
            validate_manifest(&manifest),
            Err(ProtocolError::InvalidManifest {
                message: "duplicate capability_id 'disc.chd'".to_string(),
            })
        );
    }

    #[test]
    fn rejects_incompatible_protocol_version() {
        let manifest = PluginManifest {
            protocol_version: ProtocolVersion::new(2, 0),
            ..valid_manifest()
        };

        assert_eq!(
            validate_manifest(&manifest),
            Err(ProtocolError::IncompatibleProtocolVersion {
                host: ENCODER_PLUGIN_PROTOCOL_V1,
                plugin: ProtocolVersion::new(2, 0),
            })
        );
    }

    #[test]
    fn request_validation_rejects_unknown_capability_id() {
        let manifest = valid_manifest();
        let request = MaterializationRequest {
            artifact_id: "artifact-1".to_string(),
            logical_name: "Game.chd".to_string(),
            selected_capability_id: "missing.capability".to_string(),
            artifact_kind: ProtocolArtifactKind::SourceBacked(ProtocolSourceArtifact::single(
                "file:/roms/game.bin",
                1024,
            )),
            requirements: ProtocolCapabilityRequirements::new(ProtocolContentType::Disc)
                .with_format(ProtocolFormat::Chd),
            context: ProtocolMaterializationContext::default(),
        };

        assert_eq!(
            validate_materialization_request(&request, &manifest),
            Err(ProtocolError::UnknownCapabilityId {
                capability_id: "missing.capability".to_string(),
            })
        );
    }

    #[test]
    fn request_validation_accepts_playlist_with_context() {
        let manifest = valid_manifest();
        let request = MaterializationRequest {
            artifact_id: "playlist-1".to_string(),
            logical_name: "Game.m3u".to_string(),
            selected_capability_id: "playlist.m3u".to_string(),
            artifact_kind: ProtocolArtifactKind::Generated(ProtocolGeneratedArtifact::Playlist(
                ProtocolPlaylistArtifact::new(vec![
                    ProtocolArtifactReference::new("disc-1"),
                    ProtocolArtifactReference::new("disc-2"),
                ]),
            )),
            requirements: ProtocolCapabilityRequirements::new(ProtocolContentType::Playlist)
                .with_format(ProtocolFormat::M3u),
            context: ProtocolMaterializationContext::default()
                .with_artifact_name("disc-1", "Game (Disc 1).cue")
                .with_artifact_name("disc-2", "Game (Disc 2).cue"),
        };

        assert_eq!(
            validate_materialization_request(&request, &manifest),
            Ok(())
        );
    }

    #[test]
    fn request_validation_rejects_playlist_without_context() {
        let manifest = valid_manifest();
        let request = MaterializationRequest {
            artifact_id: "playlist-1".to_string(),
            logical_name: "Game.m3u".to_string(),
            selected_capability_id: "playlist.m3u".to_string(),
            artifact_kind: ProtocolArtifactKind::Generated(ProtocolGeneratedArtifact::Playlist(
                ProtocolPlaylistArtifact::new(vec![ProtocolArtifactReference::new("disc-1")]),
            )),
            requirements: ProtocolCapabilityRequirements::new(ProtocolContentType::Playlist)
                .with_format(ProtocolFormat::M3u),
            context: ProtocolMaterializationContext::default(),
        };

        assert_eq!(
            validate_materialization_request(&request, &manifest),
            Err(ProtocolError::MissingContext {
                message: "missing artifact name for referenced artifact 'disc-1'".to_string(),
            })
        );
    }

    #[test]
    fn protocol_types_round_trip_through_json() {
        let request = MaterializationRequest {
            artifact_id: "artifact-1".to_string(),
            logical_name: "Game.chd".to_string(),
            selected_capability_id: "disc.chd".to_string(),
            artifact_kind: ProtocolArtifactKind::SourceBacked(ProtocolSourceArtifact::single(
                "file:/roms/game.bin",
                1024,
            )),
            requirements: ProtocolCapabilityRequirements::new(ProtocolContentType::Disc)
                .with_format(ProtocolFormat::Chd)
                .require_feature(ProtocolCapabilityFeature::Lossless),
            context: ProtocolMaterializationContext::default(),
        };

        let json = serde_json::to_string(&request).unwrap();
        let decoded: MaterializationRequest = serde_json::from_str(&json).unwrap();

        assert_eq!(decoded, request);
    }
}
