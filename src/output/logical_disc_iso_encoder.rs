use std::io;

use crate::output::capabilities::{CapabilityFeature, ContentType, EncoderCapability, Format};
use crate::output::encode::{MaterializationContext, MaterializedArtifact, OutputEncoder};
use crate::output::plan::{ArtifactRequest, PlannedArtifactKind};

pub struct LogicalDiscIsoEncoder;

impl LogicalDiscIsoEncoder {
    pub fn new() -> Self {
        Self
    }
}

impl Default for LogicalDiscIsoEncoder {
    fn default() -> Self {
        Self::new()
    }
}

impl OutputEncoder for LogicalDiscIsoEncoder {
    fn plugin_id(&self) -> &str {
        "logical-disc-iso"
    }

    fn capabilities(&self) -> Vec<EncoderCapability> {
        vec![
            EncoderCapability::new(self.plugin_id(), "disc.logical.iso", ContentType::Disc)
                .supports_format(Format::Iso)
                .with_feature(CapabilityFeature::RandomAccess)
                .with_feature(CapabilityFeature::Lossless),
        ]
    }

    fn materialize(
        &self,
        _file_name: &str,
        artifact: &ArtifactRequest,
        selected_capability_id: &str,
        _context: &MaterializationContext,
    ) -> Result<MaterializedArtifact, io::Error> {
        if selected_capability_id != "disc.logical.iso" {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                format!("unsupported capability '{selected_capability_id}'"),
            ));
        }

        let PlannedArtifactKind::ContentBacked(content) = &artifact.kind else {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "logical ISO encoding requires a content-backed artifact",
            ));
        };
        let crate::output::plan::ContentArtifact::LogicalDisc(disc) = content else {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "logical ISO encoding requires logical disc content",
            ));
        };

        if disc.sector_size != 2048 {
            return Err(io::Error::new(
                io::ErrorKind::Unsupported,
                "ISO output requires 2048-byte logical sectors",
            ));
        }

        let size = disc.byte_len().ok_or_else(|| {
            io::Error::new(io::ErrorKind::InvalidData, "logical disc size overflow")
        })?;

        Ok(MaterializedArtifact::ReaderBacked {
            handle: disc.content.clone(),
            size,
        })
    }
}
