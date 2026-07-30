use std::sync::Arc;

use crate::output::capabilities::EncoderCapability;
use crate::output::encode::{MaterializationContext, MaterializedArtifact, OutputEncoder};
use crate::output::plan::ArtifactRequest;
use crate::output::plugin_client::EncoderPluginClient;
use crate::output::plugin_protocol::{validate_manifest, ProtocolError};
use crate::output::plugin_protocol_conversion::{
    from_materialization_response, to_encoder_capability, to_materialization_request,
};

pub struct PluginBackedEncoder {
    plugin_id: String,
    capabilities: Vec<EncoderCapability>,
    client: Arc<dyn EncoderPluginClient>,
}

impl PluginBackedEncoder {
    pub fn new(
        plugin_id: String,
        capabilities: Vec<EncoderCapability>,
        client: Arc<dyn EncoderPluginClient>,
    ) -> Self {
        Self {
            plugin_id,
            capabilities,
            client,
        }
    }

    pub fn from_client(client: Arc<dyn EncoderPluginClient>) -> Result<Self, ProtocolError> {
        let manifest = client.manifest()?;
        validate_manifest(&manifest)?;

        let plugin_id = manifest.plugin_id.clone();
        let capabilities = manifest
            .capabilities
            .into_iter()
            .map(|capability| to_encoder_capability(&plugin_id, capability))
            .collect();

        Ok(Self {
            plugin_id,
            capabilities,
            client,
        })
    }
}

impl OutputEncoder for PluginBackedEncoder {
    fn plugin_id(&self) -> &str {
        &self.plugin_id
    }

    fn capabilities(&self) -> Vec<EncoderCapability> {
        self.capabilities.clone()
    }

    fn materialize(
        &self,
        file_name: &str,
        artifact: &ArtifactRequest,
        capability_id: &str,
        context: &MaterializationContext,
    ) -> Result<MaterializedArtifact, std::io::Error> {
        let request = to_materialization_request(file_name, artifact, capability_id, context)
            .map_err(to_io_error)?;

        let response = self.client.materialize(&request).map_err(to_io_error)?;

        from_materialization_response(response).map_err(to_io_error)
    }
}

fn to_io_error(error: ProtocolError) -> std::io::Error {
    std::io::Error::other(format!("{error:?}"))
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;
    use std::sync::Arc;

    use super::*;
    use crate::core::source::SourceRef;
    use crate::output::capabilities::{CapabilityRequirements, ContentType, Format};
    use crate::output::encode::MaterializationContext;
    use crate::output::plan::{ArtifactId, ArtifactRequest, PlannedArtifactKind, SourceArtifact};
    use crate::output::plugin_client::EncoderPluginClient;
    use crate::output::plugin_protocol::{
        MaterializationRequest, MaterializationResponse, PluginManifest, ProtocolContentType,
        ProtocolEncoderCapability, ProtocolFormat, ProtocolMaterializedSourceFile,
        ENCODER_PLUGIN_PROTOCOL_V1,
    };

    #[derive(Clone)]
    struct TestClient {
        manifest: PluginManifest,
    }

    impl EncoderPluginClient for TestClient {
        fn manifest(&self) -> Result<PluginManifest, ProtocolError> {
            Ok(self.manifest.clone())
        }

        fn materialize(
            &self,
            _request: &MaterializationRequest,
        ) -> Result<MaterializationResponse, ProtocolError> {
            Ok(MaterializationResponse::SourceBacked(
                ProtocolMaterializedSourceFile {
                    source: "file:/roms/game.iso".to_string(),
                    size: 4096,
                },
            ))
        }
    }

    fn manifest() -> PluginManifest {
        PluginManifest {
            plugin_id: "plugin.example".to_string(),
            plugin_version: "0.1.0".to_string(),
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

    #[test]
    fn plugin_encoder_integrates_with_output_encoder_trait() {
        let client: Arc<dyn EncoderPluginClient> = Arc::new(TestClient {
            manifest: manifest(),
        });

        let encoder = PluginBackedEncoder::new(
            "plugin.example".to_string(),
            vec![
                EncoderCapability::new("plugin.example", "disc.iso", ContentType::Disc)
                    .supports_format(Format::Iso),
            ],
            client,
        );

        let artifact = ArtifactRequest::new(
            ArtifactId::new("game"),
            PlannedArtifactKind::SourceBacked(SourceArtifact::single(
                SourceRef::new("file:/roms/game.iso"),
                4096,
            )),
            CapabilityRequirements::new(ContentType::Disc).with_format(Format::Iso),
        );

        let context = MaterializationContext {
            artifact_names: HashMap::new(),
        };

        let result = encoder
            .materialize("Game.iso", &artifact, "disc.iso", &context)
            .unwrap();

        assert_eq!(
            result,
            MaterializedArtifact::SourceBacked {
                source: SourceRef::new("file:/roms/game.iso"),
                size: 4096,
            }
        );
    }

    #[test]
    fn builds_plugin_encoder_from_manifest() {
        let manifest = manifest();
        let expected_plugin_id = manifest.plugin_id.clone();
        let expected_capability_count = manifest.capabilities.len();

        let client: Arc<dyn EncoderPluginClient> = Arc::new(TestClient { manifest });
        let encoder = PluginBackedEncoder::from_client(client).unwrap();

        assert_eq!(encoder.plugin_id(), expected_plugin_id);
        assert_eq!(encoder.capabilities().len(), expected_capability_count);

        let capability = &encoder.capabilities()[0];
        assert_eq!(capability.plugin_id, "plugin.example");
        assert_eq!(capability.capability_id, "disc.iso");
        assert_eq!(capability.content_type, ContentType::Disc);
        assert!(capability.formats.contains(&Format::Iso));
    }
}
