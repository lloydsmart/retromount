use std::io;

use crate::core::source_resolver::open_source_ref;
use crate::output::capabilities::{CapabilityFeature, ContentType, EncoderCapability, Format};
use crate::output::encode::{
    MaterializationContext, MaterializedArtifact, MaterializedNamedArtifact, OutputEncoder,
};
use crate::output::plan::{ArtifactRequest, ContentArtifact, PlannedArtifactKind};

pub struct ChdPassthroughEncoder;

impl ChdPassthroughEncoder {
    pub fn new() -> Self {
        Self
    }
}

impl Default for ChdPassthroughEncoder {
    fn default() -> Self {
        Self::new()
    }
}

impl OutputEncoder for ChdPassthroughEncoder {
    fn plugin_id(&self) -> &str {
        "chd-passthrough"
    }

    fn capabilities(&self) -> Vec<EncoderCapability> {
        vec![
            EncoderCapability::new(self.plugin_id(), "disc.chd-passthrough", ContentType::Disc)
                .supports_format(Format::Chd)
                .with_feature(CapabilityFeature::Lossless)
                .with_feature(CapabilityFeature::RandomAccess)
                .with_feature(CapabilityFeature::MultiFile),
        ]
    }

    fn materialize(
        &self,
        _file_name: &str,
        _artifact: &ArtifactRequest,
        selected_capability_id: &str,
        _context: &MaterializationContext,
    ) -> Result<MaterializedArtifact, io::Error> {
        Err(io::Error::new(
            io::ErrorKind::Unsupported,
            format!("capability '{selected_capability_id}' produces an artifact set"),
        ))
    }

    fn materialize_set(
        &self,
        artifact_name: &str,
        artifact: &ArtifactRequest,
        selected_capability_id: &str,
        _context: &MaterializationContext,
    ) -> Result<Vec<MaterializedNamedArtifact>, io::Error> {
        if selected_capability_id != "disc.chd-passthrough" {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                format!("unsupported capability '{selected_capability_id}'"),
            ));
        }
        let PlannedArtifactKind::ContentBacked(ContentArtifact::CdDisc(disc)) = &artifact.kind
        else {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "CHD passthrough requires track-aware CD content",
            ));
        };
        let source = disc
            .tracks
            .first()
            .map(|track| track.source.clone())
            .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "CHD has no tracks"))?;
        if !source.file_name().to_ascii_lowercase().ends_with(".chd")
            || disc.tracks.iter().any(|track| track.source != source)
        {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "CHD passthrough requires every track to originate from one CHD source",
            ));
        }
        let size = open_source_ref(&source)?.len();
        let mut members = vec![MaterializedNamedArtifact::new(
            format!("{artifact_name}.chd"),
            MaterializedArtifact::SourceBacked { source, size },
        )];
        if let Some(sbi) = &disc.sbi {
            members.push(MaterializedNamedArtifact::new(
                format!("{artifact_name}.sbi"),
                MaterializedArtifact::ReaderBacked {
                    handle: sbi.content.clone(),
                    size: sbi.size,
                },
            ));
        }
        Ok(members)
    }
}
