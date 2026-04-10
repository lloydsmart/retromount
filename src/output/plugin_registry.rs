use std::sync::Arc;

use crate::output::encode::OutputEncoder;
use crate::output::plugin_client::EncoderPluginClient;
use crate::output::plugin_encoder::PluginBackedEncoder;
use crate::output::plugin_protocol::ProtocolError;

#[derive(Default)]
pub struct PluginRegistry {
    clients: Vec<Arc<dyn EncoderPluginClient>>,
}

impl PluginRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn register(&mut self, client: Arc<dyn EncoderPluginClient>) -> Result<(), ProtocolError> {
        PluginBackedEncoder::from_client(client.clone())?;
        self.clients.push(client);
        Ok(())
    }

    pub fn build_encoders(&self) -> Result<Vec<Box<dyn OutputEncoder>>, ProtocolError> {
        let mut encoders: Vec<Box<dyn OutputEncoder>> = Vec::with_capacity(self.clients.len());

        for client in &self.clients {
            let encoder = PluginBackedEncoder::from_client(client.clone())?;
            encoders.push(Box::new(encoder));
        }

        Ok(encoders)
    }

    pub fn len(&self) -> usize {
        self.clients.len()
    }

    pub fn is_empty(&self) -> bool {
        self.clients.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use super::*;
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
                    size: 1234,
                },
            ))
        }
    }

    fn manifest() -> PluginManifest {
        PluginManifest {
            plugin_id: "plugin.test".to_string(),
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
    fn registers_valid_plugin_client() {
        let mut registry = PluginRegistry::new();

        let client: Arc<dyn EncoderPluginClient> = Arc::new(TestClient {
            manifest: manifest(),
        });

        registry.register(client).unwrap();

        assert_eq!(registry.len(), 1);
        assert!(!registry.is_empty());
    }

    #[test]
    fn builds_encoders_from_registered_clients() {
        let mut registry = PluginRegistry::new();

        let client: Arc<dyn EncoderPluginClient> = Arc::new(TestClient {
            manifest: manifest(),
        });

        registry.register(client).unwrap();

        let encoders = registry.build_encoders().unwrap();

        assert_eq!(encoders.len(), 1);
        assert_eq!(encoders[0].plugin_id(), "plugin.test");
        assert_eq!(encoders[0].capabilities().len(), 1);
    }

    #[test]
    fn rejects_invalid_plugin_manifest_during_registration() {
        let mut registry = PluginRegistry::new();

        let invalid_manifest = PluginManifest {
            plugin_id: String::new(),
            plugin_version: "0.1.0".to_string(),
            protocol_version: ENCODER_PLUGIN_PROTOCOL_V1,
            display_name: None,
            description: None,
            capabilities: vec![ProtocolEncoderCapability::new(
                "disc.iso",
                ProtocolContentType::Disc,
            )
            .supports_format(ProtocolFormat::Iso)],
        };

        let client: Arc<dyn EncoderPluginClient> = Arc::new(TestClient {
            manifest: invalid_manifest,
        });

        let error = registry.register(client).unwrap_err();

        assert_eq!(
            error,
            ProtocolError::InvalidManifest {
                message: "plugin_id must not be empty".to_string(),
            }
        );
        assert!(registry.is_empty());
    }
}
