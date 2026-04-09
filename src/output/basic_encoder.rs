use std::io;

use crate::output::capabilities::{ContentType, EncoderCapability, Format};
use crate::output::encode::{MaterializationContext, MaterializedArtifact, OutputEncoder};
use crate::output::plan::{
    ArtifactReference, ArtifactRequest, GeneratedArtifact, PlannedArtifactKind, PlaylistArtifact,
    SourceArtifact,
};

pub struct BasicEncoder;

impl BasicEncoder {
    pub fn new() -> Self {
        Self
    }

    fn materialize_source_backed(
        &self,
        artifact: &SourceArtifact,
    ) -> Result<MaterializedArtifact, io::Error> {
        match artifact.inputs.as_slice() {
            [input] => Ok(MaterializedArtifact::SourceBacked {
                source: input.source.clone(),
                size: input.size,
            }),
            _ => Err(io::Error::new(
                io::ErrorKind::Unsupported,
                "basic encoder does not support multi-source source-backed artifacts",
            )),
        }
    }

    fn materialize_playlist(
        &self,
        playlist: &PlaylistArtifact,
        context: &MaterializationContext,
    ) -> Result<MaterializedArtifact, io::Error> {
        let mut lines = Vec::with_capacity(playlist.entries.len());

        for ArtifactReference { artifact_id } in &playlist.entries {
            let name = context.artifact_names.get(artifact_id).ok_or_else(|| {
                io::Error::new(
                    io::ErrorKind::InvalidData,
                    format!("playlist references unknown artifact '{}'", artifact_id.0),
                )
            })?;

            lines.push(name.clone());
        }

        let mut text = lines.join("\n");
        text.push('\n');

        Ok(MaterializedArtifact::Inline(text.into_bytes()))
    }
}

impl Default for BasicEncoder {
    fn default() -> Self {
        Self::new()
    }
}

impl OutputEncoder for BasicEncoder {
    fn plugin_id(&self) -> &'static str {
        "basic"
    }

    fn capabilities(&self) -> Vec<EncoderCapability> {
        vec![
            EncoderCapability::new(self.plugin_id(), "bytes.bin", ContentType::Bytes)
                .supports_format(Format::Bin),
            EncoderCapability::new(self.plugin_id(), "text.txt", ContentType::Text)
                .supports_format(Format::Text),
            EncoderCapability::new(self.plugin_id(), "rom.bin", ContentType::Rom)
                .supports_format(Format::Bin),
            EncoderCapability::new(self.plugin_id(), "disc.bin", ContentType::Disc)
                .supports_format(Format::Bin),
            EncoderCapability::new(self.plugin_id(), "playlist.m3u", ContentType::Playlist)
                .supports_format(Format::M3u),
        ]
    }

    fn materialize(
        &self,
        _file_name: &str,
        artifact: &ArtifactRequest,
        selected_capability_id: &str,
        context: &MaterializationContext,
    ) -> Result<MaterializedArtifact, io::Error> {
        match (&artifact.kind, selected_capability_id) {
            (PlannedArtifactKind::SourceBacked(source), "bytes.bin")
            | (PlannedArtifactKind::SourceBacked(source), "text.txt")
            | (PlannedArtifactKind::SourceBacked(source), "rom.bin")
            | (PlannedArtifactKind::SourceBacked(source), "disc.bin") => {
                self.materialize_source_backed(source)
            }
            (
                PlannedArtifactKind::Generated(GeneratedArtifact::Playlist(playlist)),
                "playlist.m3u",
            ) => self.materialize_playlist(playlist, context),
            _ => Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                format!(
                    "basic encoder cannot materialize artifact '{}' with capability '{}'",
                    artifact.id.0, selected_capability_id
                ),
            )),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::source::SourceRef;
    use crate::output::capabilities::CapabilityRequirements;
    use crate::output::plan::{
        ArtifactId, ArtifactReference, ArtifactRequest, GeneratedArtifact, PlannedArtifactKind,
        PlaylistArtifact, SourceArtifact, SourceArtifactInput,
    };

    #[test]
    fn exposes_expected_capabilities() {
        let encoder = BasicEncoder::new();
        let capabilities = encoder.capabilities();

        let ids: Vec<&str> = capabilities
            .iter()
            .map(|c| c.capability_id.as_str())
            .collect();
        assert_eq!(
            ids,
            vec![
                "bytes.bin",
                "text.txt",
                "rom.bin",
                "disc.bin",
                "playlist.m3u"
            ]
        );
    }

    #[test]
    fn materializes_single_source_artifact() {
        let encoder = BasicEncoder::new();

        let artifact = ArtifactRequest::new(
            ArtifactId::new("rom"),
            PlannedArtifactKind::SourceBacked(SourceArtifact::single(
                SourceRef::new("file:/roms/game.bin"),
                4096,
            )),
            CapabilityRequirements::new(ContentType::Rom).with_format(Format::Bin),
        );

        let materialized = encoder
            .materialize(
                "game.bin",
                &artifact,
                "rom.bin",
                &MaterializationContext {
                    artifact_names: Default::default(),
                },
            )
            .unwrap();

        assert_eq!(
            materialized,
            MaterializedArtifact::SourceBacked {
                source: SourceRef::new("file:/roms/game.bin"),
                size: 4096,
            }
        );
    }

    #[test]
    fn materializes_playlist_inline() {
        let encoder = BasicEncoder::new();

        let artifact = ArtifactRequest::new(
            ArtifactId::new("playlist"),
            PlannedArtifactKind::Generated(GeneratedArtifact::Playlist(PlaylistArtifact::new(
                vec![
                    ArtifactReference::new(ArtifactId::new("disc1")),
                    ArtifactReference::new(ArtifactId::new("disc2")),
                ],
            ))),
            CapabilityRequirements::new(ContentType::Playlist).with_format(Format::M3u),
        );

        let mut artifact_names = std::collections::HashMap::new();
        artifact_names.insert(ArtifactId::new("disc1"), "Game (Disc 1).cue".to_string());
        artifact_names.insert(ArtifactId::new("disc2"), "Game (Disc 2).cue".to_string());

        let materialized = encoder
            .materialize(
                "Game.m3u",
                &artifact,
                "playlist.m3u",
                &MaterializationContext { artifact_names },
            )
            .unwrap();

        assert_eq!(
            materialized,
            MaterializedArtifact::Inline(b"Game (Disc 1).cue\nGame (Disc 2).cue\n".to_vec())
        );
    }

    #[test]
    fn rejects_multi_source_source_backed_artifact() {
        let encoder = BasicEncoder::new();

        let artifact = ArtifactRequest::new(
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
        );

        let err = encoder
            .materialize(
                "merged.bin",
                &artifact,
                "rom.bin",
                &MaterializationContext {
                    artifact_names: Default::default(),
                },
            )
            .unwrap_err();

        assert_eq!(err.kind(), io::ErrorKind::Unsupported);
    }
}
