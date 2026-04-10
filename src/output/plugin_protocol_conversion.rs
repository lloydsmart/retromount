use std::collections::HashMap;

use crate::core::source::SourceRef;
use crate::output::capabilities::{
    CapabilityFeature, CapabilityRequirements, ContentType, EncoderCapability, Format,
};
use crate::output::encode::{MaterializationContext, MaterializedArtifact};
use crate::output::plan::{
    ArtifactId, ArtifactReference, ArtifactRequest, GeneratedArtifact, PlannedArtifactKind,
    PlaylistArtifact, SourceArtifact, SourceArtifactInput,
};
use crate::output::plugin_protocol::{
    MaterializationRequest, MaterializationResponse, PluginManifest, ProtocolArtifactKind,
    ProtocolArtifactName, ProtocolArtifactReference, ProtocolCapabilityFeature,
    ProtocolCapabilityRequirements, ProtocolContentType, ProtocolEncoderCapability, ProtocolError,
    ProtocolFormat, ProtocolGeneratedArtifact, ProtocolInlineFile, ProtocolMaterializationContext,
    ProtocolMaterializedSourceFile, ProtocolPlaylistArtifact, ProtocolSourceArtifact,
    ProtocolSourceArtifactInput,
};

pub fn to_protocol_manifest(
    plugin_id: impl Into<String>,
    plugin_version: impl Into<String>,
    protocol_version: crate::output::plugin_protocol::ProtocolVersion,
    display_name: Option<String>,
    description: Option<String>,
    capabilities: &[EncoderCapability],
) -> PluginManifest {
    PluginManifest {
        plugin_id: plugin_id.into(),
        plugin_version: plugin_version.into(),
        protocol_version,
        display_name,
        description,
        capabilities: capabilities
            .iter()
            .cloned()
            .map(ProtocolEncoderCapability::from)
            .collect(),
    }
}

pub fn to_materialization_request(
    file_name: &str,
    artifact: &ArtifactRequest,
    selected_capability_id: &str,
    context: &MaterializationContext,
) -> MaterializationRequest {
    MaterializationRequest {
        artifact_id: artifact.id.0.clone(),
        logical_name: file_name.to_string(),
        selected_capability_id: selected_capability_id.to_string(),
        artifact_kind: ProtocolArtifactKind::from(artifact.kind.clone()),
        requirements: ProtocolCapabilityRequirements::from(artifact.requirements.clone()),
        context: ProtocolMaterializationContext::from(context.clone()),
    }
}

pub fn from_materialization_response(
    response: MaterializationResponse,
) -> Result<MaterializedArtifact, ProtocolError> {
    match response {
        MaterializationResponse::SourceBacked(file) => Ok(MaterializedArtifact::SourceBacked {
            source: SourceRef::new(file.source),
            size: file.size,
        }),
        MaterializationResponse::Inline(file) => Ok(MaterializedArtifact::Inline(file.bytes)),
    }
}

impl From<ContentType> for ProtocolContentType {
    fn from(value: ContentType) -> Self {
        match value {
            ContentType::Rom => Self::Rom,
            ContentType::Disc => Self::Disc,
            ContentType::Playlist => Self::Playlist,
            ContentType::Archive => Self::Archive,
            ContentType::Directory => Self::Directory,
            ContentType::Bytes => Self::Bytes,
            ContentType::Text => Self::Text,
            ContentType::Game => Self::Game,
        }
    }
}

impl From<ProtocolContentType> for ContentType {
    fn from(value: ProtocolContentType) -> Self {
        match value {
            ProtocolContentType::Rom => Self::Rom,
            ProtocolContentType::Disc => Self::Disc,
            ProtocolContentType::Playlist => Self::Playlist,
            ProtocolContentType::Archive => Self::Archive,
            ProtocolContentType::Directory => Self::Directory,
            ProtocolContentType::Bytes => Self::Bytes,
            ProtocolContentType::Text => Self::Text,
            ProtocolContentType::Game => Self::Game,
        }
    }
}

impl From<Format> for ProtocolFormat {
    fn from(value: Format) -> Self {
        match value {
            Format::Iso => Self::Iso,
            Format::Chd => Self::Chd,
            Format::Zip => Self::Zip,
            Format::M3u => Self::M3u,
            Format::Directory => Self::Directory,
            Format::Bin => Self::Bin,
            Format::Text => Self::Text,
        }
    }
}

impl From<ProtocolFormat> for Format {
    fn from(value: ProtocolFormat) -> Self {
        match value {
            ProtocolFormat::Iso => Self::Iso,
            ProtocolFormat::Chd => Self::Chd,
            ProtocolFormat::Zip => Self::Zip,
            ProtocolFormat::M3u => Self::M3u,
            ProtocolFormat::Directory => Self::Directory,
            ProtocolFormat::Bin => Self::Bin,
            ProtocolFormat::Text => Self::Text,
        }
    }
}

impl From<CapabilityFeature> for ProtocolCapabilityFeature {
    fn from(value: CapabilityFeature) -> Self {
        match value {
            CapabilityFeature::MultiSource => Self::MultiSource,
            CapabilityFeature::Streaming => Self::Streaming,
            CapabilityFeature::Lossless => Self::Lossless,
            CapabilityFeature::RandomAccess => Self::RandomAccess,
            CapabilityFeature::SupportsPartial => Self::SupportsPartial,
        }
    }
}

impl From<ProtocolCapabilityFeature> for CapabilityFeature {
    fn from(value: ProtocolCapabilityFeature) -> Self {
        match value {
            ProtocolCapabilityFeature::MultiSource => Self::MultiSource,
            ProtocolCapabilityFeature::Streaming => Self::Streaming,
            ProtocolCapabilityFeature::Lossless => Self::Lossless,
            ProtocolCapabilityFeature::RandomAccess => Self::RandomAccess,
            ProtocolCapabilityFeature::SupportsPartial => Self::SupportsPartial,
        }
    }
}

impl From<EncoderCapability> for ProtocolEncoderCapability {
    fn from(value: EncoderCapability) -> Self {
        let mut capability =
            ProtocolEncoderCapability::new(value.capability_id, value.content_type.into())
                .with_priority(value.priority);

        for format in value.formats {
            capability = capability.supports_format(format.into());
        }

        for feature in value.features {
            capability = capability.with_feature(feature.into());
        }

        capability
    }
}

impl From<CapabilityRequirements> for ProtocolCapabilityRequirements {
    fn from(value: CapabilityRequirements) -> Self {
        let mut requirements = ProtocolCapabilityRequirements::new(value.content_type.into());

        if let Some(format) = value.format {
            requirements = requirements.with_format(format.into());
        }

        for feature in value.required_features {
            requirements = requirements.require_feature(feature.into());
        }

        for feature in value.preferred_features {
            requirements = requirements.prefer_feature(feature.into());
        }

        for feature in value.forbidden_features {
            requirements = requirements.forbid_feature(feature.into());
        }

        requirements
    }
}

impl From<SourceArtifactInput> for ProtocolSourceArtifactInput {
    fn from(value: SourceArtifactInput) -> Self {
        Self {
            source: value.source.0.as_ref().to_string(),
            size: value.size,
        }
    }
}

impl From<SourceArtifact> for ProtocolSourceArtifact {
    fn from(value: SourceArtifact) -> Self {
        Self::multiple(
            value
                .inputs
                .into_iter()
                .map(ProtocolSourceArtifactInput::from)
                .collect(),
        )
    }
}

impl From<ArtifactReference> for ProtocolArtifactReference {
    fn from(value: ArtifactReference) -> Self {
        Self::new(value.artifact_id.0)
    }
}

impl From<PlaylistArtifact> for ProtocolPlaylistArtifact {
    fn from(value: PlaylistArtifact) -> Self {
        Self::new(
            value
                .entries
                .into_iter()
                .map(ProtocolArtifactReference::from)
                .collect(),
        )
    }
}

impl From<GeneratedArtifact> for ProtocolGeneratedArtifact {
    fn from(value: GeneratedArtifact) -> Self {
        match value {
            GeneratedArtifact::Playlist(playlist) => Self::Playlist(playlist.into()),
        }
    }
}

impl From<PlannedArtifactKind> for ProtocolArtifactKind {
    fn from(value: PlannedArtifactKind) -> Self {
        match value {
            PlannedArtifactKind::SourceBacked(source_artifact) => {
                Self::SourceBacked(source_artifact.into())
            }
            PlannedArtifactKind::Generated(generated_artifact) => {
                Self::Generated(generated_artifact.into())
            }
        }
    }
}

impl From<MaterializationContext> for ProtocolMaterializationContext {
    fn from(value: MaterializationContext) -> Self {
        Self {
            artifact_names: value
                .artifact_names
                .into_iter()
                .map(|(artifact_id, logical_name)| ProtocolArtifactName {
                    artifact_id: artifact_id.0,
                    logical_name,
                })
                .collect(),
        }
    }
}

impl TryFrom<ProtocolCapabilityRequirements> for CapabilityRequirements {
    type Error = ProtocolError;

    fn try_from(value: ProtocolCapabilityRequirements) -> Result<Self, Self::Error> {
        let mut requirements = CapabilityRequirements::new(value.content_type.into());

        if let Some(format) = value.format {
            requirements = requirements.with_format(format.into());
        }

        for feature in value.required_features {
            requirements = requirements.require_feature(feature.into());
        }

        for feature in value.preferred_features {
            requirements = requirements.prefer_feature(feature.into());
        }

        for feature in value.forbidden_features {
            requirements = requirements.forbid_feature(feature.into());
        }

        Ok(requirements)
    }
}

impl TryFrom<ProtocolSourceArtifactInput> for SourceArtifactInput {
    type Error = ProtocolError;

    fn try_from(value: ProtocolSourceArtifactInput) -> Result<Self, Self::Error> {
        if value.source.trim().is_empty() {
            return Err(ProtocolError::InvalidRequest {
                message: "source artifact input source must not be empty".to_string(),
            });
        }

        Ok(Self {
            source: SourceRef::new(value.source),
            size: value.size,
        })
    }
}

impl TryFrom<ProtocolSourceArtifact> for SourceArtifact {
    type Error = ProtocolError;

    fn try_from(value: ProtocolSourceArtifact) -> Result<Self, Self::Error> {
        let inputs: Result<Vec<_>, _> = value
            .inputs
            .into_iter()
            .map(SourceArtifactInput::try_from)
            .collect();

        let inputs = inputs?;

        if inputs.is_empty() {
            return Err(ProtocolError::InvalidRequest {
                message: "source-backed artifact must include at least one input".to_string(),
            });
        }

        Ok(SourceArtifact::multiple(inputs))
    }
}

impl From<ProtocolArtifactReference> for ArtifactReference {
    fn from(value: ProtocolArtifactReference) -> Self {
        Self::new(ArtifactId::new(value.artifact_id))
    }
}

impl TryFrom<ProtocolPlaylistArtifact> for PlaylistArtifact {
    type Error = ProtocolError;

    fn try_from(value: ProtocolPlaylistArtifact) -> Result<Self, Self::Error> {
        if value.entries.is_empty() {
            return Err(ProtocolError::InvalidRequest {
                message: "playlist artifact must include at least one entry".to_string(),
            });
        }

        Ok(PlaylistArtifact::new(
            value
                .entries
                .into_iter()
                .map(ArtifactReference::from)
                .collect(),
        ))
    }
}

impl TryFrom<ProtocolGeneratedArtifact> for GeneratedArtifact {
    type Error = ProtocolError;

    fn try_from(value: ProtocolGeneratedArtifact) -> Result<Self, Self::Error> {
        match value {
            ProtocolGeneratedArtifact::Playlist(playlist) => {
                Ok(GeneratedArtifact::Playlist(playlist.try_into()?))
            }
        }
    }
}

impl TryFrom<ProtocolArtifactKind> for PlannedArtifactKind {
    type Error = ProtocolError;

    fn try_from(value: ProtocolArtifactKind) -> Result<Self, Self::Error> {
        match value {
            ProtocolArtifactKind::SourceBacked(source_artifact) => {
                Ok(Self::SourceBacked(source_artifact.try_into()?))
            }
            ProtocolArtifactKind::Generated(generated_artifact) => {
                Ok(Self::Generated(generated_artifact.try_into()?))
            }
        }
    }
}

impl TryFrom<ProtocolMaterializationContext> for MaterializationContext {
    type Error = ProtocolError;

    fn try_from(value: ProtocolMaterializationContext) -> Result<Self, Self::Error> {
        let mut artifact_names = HashMap::new();

        for entry in value.artifact_names {
            if entry.artifact_id.trim().is_empty() {
                return Err(ProtocolError::InvalidRequest {
                    message: "artifact context entry artifact_id must not be empty".to_string(),
                });
            }

            if entry.logical_name.trim().is_empty() {
                return Err(ProtocolError::InvalidRequest {
                    message: "artifact context entry logical_name must not be empty".to_string(),
                });
            }

            artifact_names.insert(ArtifactId::new(entry.artifact_id), entry.logical_name);
        }

        Ok(MaterializationContext { artifact_names })
    }
}

pub fn to_artifact_request(
    request: MaterializationRequest,
) -> Result<(String, ArtifactRequest, String, MaterializationContext), ProtocolError> {
    let logical_name = request.logical_name;

    let artifact = ArtifactRequest::new(
        ArtifactId::new(request.artifact_id),
        request.artifact_kind.try_into()?,
        request.requirements.try_into()?,
    );

    let selected_capability_id = request.selected_capability_id;
    let context = request.context.try_into()?;

    Ok((logical_name, artifact, selected_capability_id, context))
}

pub fn to_protocol_response(artifact: MaterializedArtifact) -> MaterializationResponse {
    match artifact {
        MaterializedArtifact::SourceBacked { source, size } => {
            MaterializationResponse::SourceBacked(ProtocolMaterializedSourceFile {
                source: source.0.as_ref().to_string(),
                size,
            })
        }
        MaterializedArtifact::Inline(bytes) => {
            MaterializationResponse::Inline(ProtocolInlineFile { bytes })
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn converts_encoder_capability_to_protocol_capability() {
        let capability = EncoderCapability::new("builtin.chd", "disc.chd", ContentType::Disc)
            .supports_format(Format::Chd)
            .with_feature(CapabilityFeature::Lossless)
            .with_priority(100);

        let protocol = ProtocolEncoderCapability::from(capability);

        assert_eq!(protocol.capability_id, "disc.chd");
        assert_eq!(protocol.content_type, ProtocolContentType::Disc);
        assert!(protocol.formats.contains(&ProtocolFormat::Chd));
        assert!(protocol
            .features
            .contains(&ProtocolCapabilityFeature::Lossless));
        assert_eq!(protocol.priority, 100);
    }

    #[test]
    fn converts_artifact_request_to_materialization_request() {
        let artifact = ArtifactRequest::new(
            ArtifactId::new("playlist-1"),
            PlannedArtifactKind::Generated(GeneratedArtifact::Playlist(PlaylistArtifact::new(
                vec![
                    ArtifactReference::new(ArtifactId::new("disc-1")),
                    ArtifactReference::new(ArtifactId::new("disc-2")),
                ],
            ))),
            CapabilityRequirements::new(ContentType::Playlist).with_format(Format::M3u),
        );

        let mut artifact_names = HashMap::new();
        artifact_names.insert(ArtifactId::new("disc-1"), "Game (Disc 1).cue".to_string());
        artifact_names.insert(ArtifactId::new("disc-2"), "Game (Disc 2).cue".to_string());

        let context = MaterializationContext { artifact_names };

        let request = to_materialization_request("Game.m3u", &artifact, "playlist.m3u", &context);

        assert_eq!(request.artifact_id, "playlist-1");
        assert_eq!(request.logical_name, "Game.m3u");
        assert_eq!(request.selected_capability_id, "playlist.m3u");
        assert_eq!(
            request.requirements,
            ProtocolCapabilityRequirements::new(ProtocolContentType::Playlist)
                .with_format(ProtocolFormat::M3u)
        );

        match request.artifact_kind {
            ProtocolArtifactKind::Generated(ProtocolGeneratedArtifact::Playlist(playlist)) => {
                assert_eq!(playlist.entries.len(), 2);
                assert_eq!(playlist.entries[0].artifact_id, "disc-1");
                assert_eq!(playlist.entries[1].artifact_id, "disc-2");
            }
            other => panic!("expected generated playlist artifact, got {other:?}"),
        }

        assert_eq!(
            request.context.artifact_name_for("disc-1"),
            Some("Game (Disc 1).cue")
        );
        assert_eq!(
            request.context.artifact_name_for("disc-2"),
            Some("Game (Disc 2).cue")
        );
    }

    #[test]
    fn converts_materialization_response_to_materialized_artifact() {
        let response = MaterializationResponse::Inline(ProtocolInlineFile {
            bytes: b"hello".to_vec(),
        });

        let artifact = from_materialization_response(response).unwrap();

        assert_eq!(artifact, MaterializedArtifact::Inline(b"hello".to_vec()));
    }

    #[test]
    fn round_trips_request_through_protocol_conversion() {
        let artifact = ArtifactRequest::new(
            ArtifactId::new("game-1"),
            PlannedArtifactKind::SourceBacked(SourceArtifact::single(
                SourceRef::new("file:/roms/game.iso"),
                4096,
            )),
            CapabilityRequirements::new(ContentType::Disc)
                .with_format(Format::Iso)
                .require_feature(CapabilityFeature::Lossless),
        );

        let mut artifact_names = HashMap::new();
        artifact_names.insert(ArtifactId::new("game-1"), "Game.iso".to_string());

        let context = MaterializationContext { artifact_names };

        let protocol_request =
            to_materialization_request("Game.iso", &artifact, "disc.iso", &context);

        let (logical_name, decoded_artifact, selected_capability_id, decoded_context) =
            to_artifact_request(protocol_request).unwrap();

        assert_eq!(logical_name, "Game.iso");
        assert_eq!(decoded_artifact, artifact);
        assert_eq!(selected_capability_id, "disc.iso");
        assert_eq!(decoded_context.artifact_names, context.artifact_names);
    }

    #[test]
    fn rejects_empty_protocol_source_when_decoding_request() {
        let request = MaterializationRequest {
            artifact_id: "artifact-1".to_string(),
            logical_name: "Game.iso".to_string(),
            selected_capability_id: "disc.iso".to_string(),
            artifact_kind: ProtocolArtifactKind::SourceBacked(ProtocolSourceArtifact::multiple(
                vec![ProtocolSourceArtifactInput {
                    source: String::new(),
                    size: 123,
                }],
            )),
            requirements: ProtocolCapabilityRequirements::new(ProtocolContentType::Disc)
                .with_format(ProtocolFormat::Iso),
            context: ProtocolMaterializationContext::default(),
        };

        match to_artifact_request(request) {
            Ok(_) => panic!("expected invalid request error"),
            Err(error) => {
                assert_eq!(
                    error,
                    ProtocolError::InvalidRequest {
                        message: "source artifact input source must not be empty".to_string(),
                    }
                );
            }
        }
    }
}
