use std::collections::HashMap;
use std::io;

use crate::core::vfs::{VfsDirectory, VfsNode};
use crate::output::capabilities::EncoderCapability;
use crate::output::encode::{MaterializationContext, OutputEncoder};
use crate::output::encoder_registry::default_encoder_registry;
use crate::output::plan::{ArtifactId, PlanEntry, PlanFile, PresentationPlan};
use crate::output::plugin_registry::PluginRegistry;
use crate::output::resolution::{CapabilityResolver, ResolutionResult};

pub fn materialize_plan(plan: &PresentationPlan) -> Result<VfsDirectory, io::Error> {
    let encoders = default_encoder_registry().all();
    materialize_plan_with_encoders(plan, &encoders)
}

pub fn materialize_plan_with_plugins(
    plan: &PresentationPlan,
    plugin_registry: &PluginRegistry,
) -> Result<VfsDirectory, io::Error> {
    let builtin_encoders = default_encoder_registry().all();
    let plugin_encoders = plugin_registry
        .build_encoders()
        .map_err(|error| io::Error::other(format!("{error:?}")))?;

    let encoders = merge_encoders(builtin_encoders, plugin_encoders);
    materialize_plan_with_encoders(plan, &encoders)
}

pub fn materialize_plan_with_encoders(
    plan: &PresentationPlan,
    encoders: &[Box<dyn OutputEncoder>],
) -> Result<VfsDirectory, io::Error> {
    let artifact_names = collect_artifact_names(&plan.entries);
    let children = materialize_entries(&plan.entries, &artifact_names, encoders)?;
    Ok(VfsDirectory::with_children("", children))
}

fn merge_encoders(
    mut builtin: Vec<Box<dyn OutputEncoder>>,
    plugins: Vec<Box<dyn OutputEncoder>>,
) -> Vec<Box<dyn OutputEncoder>> {
    builtin.extend(plugins);
    builtin
}

fn collect_artifact_names(entries: &[PlanEntry]) -> HashMap<ArtifactId, String> {
    let mut names = HashMap::new();
    collect_artifact_names_recursive(entries, &mut names);
    names
}

fn collect_artifact_names_recursive(
    entries: &[PlanEntry],
    names: &mut HashMap<ArtifactId, String>,
) {
    for entry in entries {
        match entry {
            PlanEntry::Directory(dir) => collect_artifact_names_recursive(&dir.entries, names),
            PlanEntry::File(file) => {
                names.insert(file.artifact.id.clone(), file.name.clone());
            }
        }
    }
}

fn materialize_entries(
    entries: &[PlanEntry],
    artifact_names: &HashMap<ArtifactId, String>,
    encoders: &[Box<dyn OutputEncoder>],
) -> Result<Vec<VfsNode>, io::Error> {
    let mut children = Vec::with_capacity(entries.len());

    for entry in entries {
        match entry {
            PlanEntry::Directory(dir) => {
                let nested_children = materialize_entries(&dir.entries, artifact_names, encoders)?;
                children.push(VfsNode::Directory(VfsDirectory::with_children(
                    &dir.name,
                    nested_children,
                )));
            }
            PlanEntry::File(file) => {
                children.push(VfsNode::File(materialize_file(
                    file,
                    artifact_names,
                    encoders,
                )?));
            }
        }
    }

    Ok(children)
}

fn materialize_file(
    file: &PlanFile,
    artifact_names: &HashMap<ArtifactId, String>,
    encoders: &[Box<dyn OutputEncoder>],
) -> Result<crate::core::vfs::VfsFile, io::Error> {
    let capabilities = collect_capabilities(encoders);
    let resolver = CapabilityResolver;

    let selected = match resolver.resolve(&file.artifact, &capabilities) {
        ResolutionResult::Resolved { selected, .. } => selected,
        ResolutionResult::Unresolved { diagnostics } => {
            return Err(io::Error::new(
                io::ErrorKind::Unsupported,
                format!(
                    "no encoder could materialize artifact '{}' ({:?})",
                    file.name, diagnostics
                ),
            ))
        }
    };

    let encoder = encoders
        .iter()
        .find(|encoder| encoder.plugin_id() == selected.plugin_id)
        .ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::NotFound,
                format!("selected encoder '{}' is unavailable", selected.plugin_id),
            )
        })?;

    let materialized = encoder
        .materialize(
            &file.name,
            &file.artifact,
            &selected.capability_id,
            &MaterializationContext {
                artifact_names: artifact_names.clone(),
            },
        )
        .map_err(|err| {
            io::Error::new(
                err.kind(),
                format!(
                    "failed to materialize artifact '{}' (id '{}') using encoder '{}' capability '{}': {}",
                    file.name,
                    file.artifact.id.0,
                    selected.plugin_id,
                    selected.capability_id,
                    err
                ),
            )
        })?;

    Ok(materialized.to_vfs_file(&file.name))
}

fn collect_capabilities(encoders: &[Box<dyn OutputEncoder>]) -> Vec<EncoderCapability> {
    encoders
        .iter()
        .flat_map(|encoder| encoder.capabilities())
        .collect()
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use super::*;
    use crate::core::content::{DiscMedia, LogicalDisc};
    use crate::core::reader_handle::ReaderHandle;
    use crate::core::source::SourceRef;
    use crate::core::vfs_reader::open_vfs_file;
    use crate::output::basic_encoder::BasicEncoder;
    use crate::output::capabilities::{
        CapabilityFeature, CapabilityRequirements, ContentType, Format,
    };
    use crate::output::plan::{
        ArtifactReference, ArtifactRequest, ContentArtifact, GeneratedArtifact, PlanFile,
        PlannedArtifactKind, PlaylistArtifact, SourceArtifact, SourceArtifactInput,
    };
    use crate::output::plugin_client::EncoderPluginClient;
    use crate::output::plugin_protocol::{
        MaterializationRequest, MaterializationResponse, PluginManifest, ProtocolContentType,
        ProtocolEncoderCapability, ProtocolFormat, ProtocolMaterializedSourceFile,
        ENCODER_PLUGIN_PROTOCOL_V1,
    };
    use crate::readers::inline_reader::InlineReader;

    #[derive(Clone)]
    struct TestPluginClient {
        manifest: PluginManifest,
    }

    impl EncoderPluginClient for TestPluginClient {
        fn manifest(
            &self,
        ) -> Result<PluginManifest, crate::output::plugin_protocol::ProtocolError> {
            Ok(self.manifest.clone())
        }

        fn materialize(
            &self,
            _request: &MaterializationRequest,
        ) -> Result<MaterializationResponse, crate::output::plugin_protocol::ProtocolError>
        {
            Ok(MaterializationResponse::SourceBacked(
                ProtocolMaterializedSourceFile {
                    source: "file:/plugins/game.iso".to_string(),
                    size: 8192,
                },
            ))
        }
    }

    fn plugin_manifest() -> PluginManifest {
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
    fn materializes_source_backed_file() {
        let plan = PresentationPlan::new(vec![PlanEntry::File(PlanFile::new(
            "game.bin",
            ArtifactRequest::new(
                ArtifactId::new("game"),
                PlannedArtifactKind::SourceBacked(SourceArtifact::single(
                    SourceRef::new("file:/roms/game.bin"),
                    1024,
                )),
                CapabilityRequirements::new(ContentType::Rom).with_format(Format::Bin),
            ),
        ))]);

        let root = materialize_plan(&plan).unwrap();

        assert_eq!(root.children().len(), 1);
        assert_eq!(root.children()[0].name(), "game.bin");
    }

    #[test]
    fn materializes_playlist_file_from_referenced_artifact_names() {
        let disc1_id = ArtifactId::new("disc1");
        let disc2_id = ArtifactId::new("disc2");

        let plan = PresentationPlan::new(vec![
            PlanEntry::File(PlanFile::new(
                "Game (Disc 1).cue",
                ArtifactRequest::new(
                    disc1_id.clone(),
                    PlannedArtifactKind::SourceBacked(SourceArtifact::single(
                        SourceRef::new("file:/roms/disc1.cue"),
                        100,
                    )),
                    CapabilityRequirements::new(ContentType::Disc).with_format(Format::Bin),
                ),
            )),
            PlanEntry::File(PlanFile::new(
                "Game (Disc 2).cue",
                ArtifactRequest::new(
                    disc2_id.clone(),
                    PlannedArtifactKind::SourceBacked(SourceArtifact::single(
                        SourceRef::new("file:/roms/disc2.cue"),
                        200,
                    )),
                    CapabilityRequirements::new(ContentType::Disc).with_format(Format::Bin),
                ),
            )),
            PlanEntry::File(PlanFile::new(
                "Game.m3u",
                ArtifactRequest::new(
                    ArtifactId::new("playlist"),
                    PlannedArtifactKind::Generated(GeneratedArtifact::Playlist(
                        PlaylistArtifact::new(vec![
                            ArtifactReference::new(disc1_id),
                            ArtifactReference::new(disc2_id),
                        ]),
                    )),
                    CapabilityRequirements::new(ContentType::Playlist).with_format(Format::M3u),
                ),
            )),
        ]);

        let root = materialize_plan(&plan).unwrap();

        let playlist = match &root.children()[2] {
            VfsNode::File(file) => file,
            other => panic!("expected file, got {other:?}"),
        };

        match &playlist.backing {
            crate::core::vfs::FileBacking::Inline(contents) => {
                assert_eq!(contents, b"Game (Disc 1).cue\nGame (Disc 2).cue\n");
            }
            other => panic!("expected inline backing, got {other:?}"),
        }
    }

    #[test]
    fn rejects_multi_source_source_backed_artifact() {
        let plan = PresentationPlan::new(vec![PlanEntry::File(PlanFile::new(
            "merged.bin",
            ArtifactRequest::new(
                ArtifactId::new("merged"),
                PlannedArtifactKind::SourceBacked(SourceArtifact::multiple(vec![
                    SourceArtifactInput {
                        source: SourceRef::new("file:/roms/part1.bin"),
                        size: 100,
                    },
                    SourceArtifactInput {
                        source: SourceRef::new("file:/roms/part2.bin"),
                        size: 200,
                    },
                ])),
                CapabilityRequirements::new(ContentType::Rom).with_format(Format::Bin),
            ),
        ))]);

        let error = materialize_plan(&plan).unwrap_err();
        assert_eq!(error.kind(), io::ErrorKind::Unsupported);
    }

    #[test]
    fn materializes_plan_with_explicit_encoder_set() {
        let plan = PresentationPlan::new(vec![PlanEntry::File(PlanFile::new(
            "game.bin",
            ArtifactRequest::new(
                ArtifactId::new("game"),
                PlannedArtifactKind::SourceBacked(SourceArtifact::single(
                    SourceRef::new("file:/roms/game.bin"),
                    1024,
                )),
                CapabilityRequirements::new(ContentType::Rom).with_format(Format::Bin),
            ),
        ))]);

        let encoders: Vec<Box<dyn OutputEncoder>> = vec![Box::new(BasicEncoder::new())];

        let root = materialize_plan_with_encoders(&plan, &encoders).unwrap();

        assert_eq!(root.children().len(), 1);
        assert_eq!(root.children()[0].name(), "game.bin");
    }

    #[test]
    fn materializes_logical_dvd_as_live_random_access_iso() {
        let mut bytes = vec![0; 4096];
        bytes[32..36].copy_from_slice(b"head");
        bytes[3000..3004].copy_from_slice(b"tail");
        let reader_bytes = bytes.clone();
        let plan = PresentationPlan::new(vec![PlanEntry::File(PlanFile::new(
            "Game.iso",
            ArtifactRequest::new(
                ArtifactId::new("game"),
                PlannedArtifactKind::ContentBacked(ContentArtifact::logical_disc(LogicalDisc {
                    media: DiscMedia::Dvd,
                    sector_size: 2048,
                    sector_count: 2,
                    content: ReaderHandle::new("test:logical-dvd", move || {
                        Ok(Box::new(InlineReader::new(reader_bytes.clone())))
                    }),
                })),
                CapabilityRequirements::new(ContentType::Disc)
                    .with_format(Format::Iso)
                    .require_feature(CapabilityFeature::RandomAccess)
                    .require_feature(CapabilityFeature::Lossless),
            ),
        ))]);

        let root = materialize_plan(&plan).unwrap();
        let file = root.find_file("Game.iso").unwrap();
        assert_eq!(file.size, 4096);

        let mut reader = open_vfs_file(file).unwrap();
        let mut tail = [0; 4];
        let mut head = [0; 4];
        reader.read_at(3000, &mut tail).unwrap();
        reader.read_at(32, &mut head).unwrap();

        assert_eq!(&tail, b"tail");
        assert_eq!(&head, b"head");
    }

    #[test]
    fn materializes_plan_with_plugin_registry() {
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

        let mut plugin_registry = PluginRegistry::new();
        let client: Arc<dyn EncoderPluginClient> = Arc::new(TestPluginClient {
            manifest: plugin_manifest(),
        });
        plugin_registry.register(client).unwrap();

        let root = materialize_plan_with_plugins(&plan, &plugin_registry).unwrap();

        assert_eq!(root.children().len(), 1);

        let file = match &root.children()[0] {
            VfsNode::File(file) => file,
            other => panic!("expected file, got {other:?}"),
        };

        match &file.backing {
            crate::core::vfs::FileBacking::Source(source) => {
                assert_eq!(source, &SourceRef::new("file:/plugins/game.iso"));
                assert_eq!(file.size, 8192);
            }
            other => panic!("expected source-backed file, got {other:?}"),
        }
    }
}
