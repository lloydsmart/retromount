use crate::output::encode::{MaterializationContext, MaterializedArtifact};
use crate::output::plan::ArtifactRequest;
use crate::output::plugin_protocol::{
    validate_manifest, validate_materialization_request, MaterializationRequest,
    MaterializationResponse, PluginManifest, ProtocolError,
};
use crate::output::plugin_protocol_conversion::{
    from_materialization_response, to_materialization_request,
};

pub trait EncoderPluginClient: Send + Sync {
    fn manifest(&self) -> Result<PluginManifest, ProtocolError>;

    fn materialize(
        &self,
        request: &MaterializationRequest,
    ) -> Result<MaterializationResponse, ProtocolError>;
}

pub fn materialize_via_plugin(
    client: &dyn EncoderPluginClient,
    file_name: &str,
    artifact: &ArtifactRequest,
    selected_capability_id: &str,
    context: &MaterializationContext,
) -> Result<MaterializedArtifact, ProtocolError> {
    let manifest = client.manifest()?;
    validate_manifest(&manifest)?;

    let request = to_materialization_request(file_name, artifact, selected_capability_id, context)?;

    validate_materialization_request(&request, &manifest)?;

    let response = client.materialize(&request)?;
    from_materialization_response(response)
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use super::*;
    use crate::core::source::SourceRef;
    use crate::output::capabilities::{CapabilityRequirements, ContentType, Format};
    use crate::output::encode::MaterializationContext;
    use crate::output::plan::{
        ArtifactId, ArtifactReference, ArtifactRequest, GeneratedArtifact, PlannedArtifactKind,
        PlaylistArtifact, SourceArtifact,
    };
    use crate::output::plugin_protocol::{
        PluginManifest, ProtocolCapabilityFeature, ProtocolContentType, ProtocolEncoderCapability,
        ProtocolFormat, ProtocolInlineFile, ProtocolMaterializedSourceFile,
        ENCODER_PLUGIN_PROTOCOL_V1,
    };

    #[derive(Debug, Clone)]
    struct StaticEncoderPluginClient {
        manifest: PluginManifest,
        response: MaterializationResponse,
    }

    impl StaticEncoderPluginClient {
        fn new(manifest: PluginManifest, response: MaterializationResponse) -> Self {
            Self { manifest, response }
        }
    }

    impl EncoderPluginClient for StaticEncoderPluginClient {
        fn manifest(&self) -> Result<PluginManifest, ProtocolError> {
            Ok(self.manifest.clone())
        }

        fn materialize(
            &self,
            _request: &MaterializationRequest,
        ) -> Result<MaterializationResponse, ProtocolError> {
            Ok(self.response.clone())
        }
    }

    fn disc_manifest() -> PluginManifest {
        PluginManifest {
            plugin_id: "plugin.example".to_string(),
            plugin_version: "0.1.0".to_string(),
            protocol_version: ENCODER_PLUGIN_PROTOCOL_V1,
            display_name: Some("Example Plugin".to_string()),
            description: Some("Example disc encoder".to_string()),
            capabilities: vec![ProtocolEncoderCapability::new(
                "disc.iso",
                ProtocolContentType::Disc,
            )
            .supports_format(ProtocolFormat::Iso)
            .with_feature(ProtocolCapabilityFeature::Lossless)],
        }
    }

    fn playlist_manifest() -> PluginManifest {
        PluginManifest {
            plugin_id: "plugin.example".to_string(),
            plugin_version: "0.1.0".to_string(),
            protocol_version: ENCODER_PLUGIN_PROTOCOL_V1,
            display_name: Some("Example Plugin".to_string()),
            description: Some("Example playlist encoder".to_string()),
            capabilities: vec![ProtocolEncoderCapability::new(
                "playlist.m3u",
                ProtocolContentType::Playlist,
            )
            .supports_format(ProtocolFormat::M3u)
            .with_feature(ProtocolCapabilityFeature::MultiSource)],
        }
    }

    #[test]
    fn materializes_source_backed_artifact_via_plugin_client() {
        let client = StaticEncoderPluginClient::new(
            disc_manifest(),
            MaterializationResponse::SourceBacked(ProtocolMaterializedSourceFile {
                source: "file:/roms/game.iso".to_string(),
                size: 4096,
            }),
        );

        let artifact = ArtifactRequest::new(
            ArtifactId::new("game-1"),
            PlannedArtifactKind::SourceBacked(SourceArtifact::single(
                SourceRef::new("file:/roms/game.iso"),
                4096,
            )),
            CapabilityRequirements::new(ContentType::Disc).with_format(Format::Iso),
        );

        let context = MaterializationContext {
            artifact_names: HashMap::new(),
        };

        let materialized =
            materialize_via_plugin(&client, "Game.iso", &artifact, "disc.iso", &context).unwrap();

        assert_eq!(
            materialized,
            MaterializedArtifact::SourceBacked {
                source: SourceRef::new("file:/roms/game.iso"),
                size: 4096,
            }
        );
    }

    #[test]
    fn rejects_request_when_selected_capability_is_unknown() {
        let client = StaticEncoderPluginClient::new(
            disc_manifest(),
            MaterializationResponse::SourceBacked(ProtocolMaterializedSourceFile {
                source: "file:/roms/game.iso".to_string(),
                size: 4096,
            }),
        );

        let artifact = ArtifactRequest::new(
            ArtifactId::new("game-1"),
            PlannedArtifactKind::SourceBacked(SourceArtifact::single(
                SourceRef::new("file:/roms/game.iso"),
                4096,
            )),
            CapabilityRequirements::new(ContentType::Disc).with_format(Format::Iso),
        );

        let context = MaterializationContext {
            artifact_names: HashMap::new(),
        };

        let error = materialize_via_plugin(
            &client,
            "Game.iso",
            &artifact,
            "missing.capability",
            &context,
        )
        .unwrap_err();

        assert_eq!(
            error,
            ProtocolError::UnknownCapabilityId {
                capability_id: "missing.capability".to_string(),
            }
        );
    }

    #[test]
    fn materializes_inline_playlist_output_via_plugin_client() {
        let client = StaticEncoderPluginClient::new(
            playlist_manifest(),
            MaterializationResponse::Inline(ProtocolInlineFile {
                bytes: b"Game (Disc 1).cue\nGame (Disc 2).cue\n".to_vec(),
            }),
        );

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

        let materialized =
            materialize_via_plugin(&client, "Game.m3u", &artifact, "playlist.m3u", &context)
                .unwrap();

        assert_eq!(
            materialized,
            MaterializedArtifact::Inline(b"Game (Disc 1).cue\nGame (Disc 2).cue\n".to_vec())
        );
    }
}
