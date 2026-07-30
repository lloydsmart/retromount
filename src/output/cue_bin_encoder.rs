use std::io;

use crate::core::cd::{CdSectorFormat, CdTrack};
use crate::core::reader_handle::ReaderHandle;
use crate::output::capabilities::{CapabilityFeature, ContentType, EncoderCapability, Format};
use crate::output::encode::{
    MaterializationContext, MaterializedArtifact, MaterializedNamedArtifact, OutputEncoder,
};
use crate::output::plan::{ArtifactRequest, ContentArtifact, PlannedArtifactKind};
use crate::readers::range_reader::RangeReader;

pub struct CueBinEncoder;

impl CueBinEncoder {
    pub fn new() -> Self {
        Self
    }
}

impl Default for CueBinEncoder {
    fn default() -> Self {
        Self::new()
    }
}

impl OutputEncoder for CueBinEncoder {
    fn plugin_id(&self) -> &str {
        "cue-bin"
    }

    fn capabilities(&self) -> Vec<EncoderCapability> {
        vec![
            EncoderCapability::new(self.plugin_id(), "disc.cue-bin", ContentType::Disc)
                .supports_format(Format::CueBin)
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
        if selected_capability_id != "disc.cue-bin" {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                format!("unsupported capability '{selected_capability_id}'"),
            ));
        }
        if artifact_name.is_empty() || artifact_name.contains('"') {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "CUE/BIN artifact name must be non-empty and contain no quotes",
            ));
        }
        let PlannedArtifactKind::ContentBacked(ContentArtifact::CdDisc(disc)) = &artifact.kind
        else {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "CUE/BIN encoding requires track-aware CD content",
            ));
        };
        if disc.tracks.is_empty() {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "CUE/BIN encoding requires at least one track",
            ));
        }

        let mut cue = String::new();
        let mut members = Vec::with_capacity(disc.tracks.len() + 2);
        for track in &disc.tracks {
            validate_track(track)?;
            let bin_name = format!("{artifact_name} (Track {:02}).bin", track.number);
            cue.push_str(&format!("FILE \"{bin_name}\" BINARY\n"));
            cue.push_str(&format!(
                "  TRACK {:02} {}\n",
                track.number,
                cue_track_mode(track.sector_format)
            ));
            if track.declared_pregap_sectors > 0 {
                cue.push_str(&format!(
                    "    PREGAP {}\n",
                    cue_time(track.declared_pregap_sectors)?
                ));
            }
            for index in &track.indexes {
                cue.push_str(&format!(
                    "    INDEX {:02} {}\n",
                    index.number,
                    cue_time(index.sector)?
                ));
            }

            let size = track.encoded_len().ok_or_else(|| {
                io::Error::new(io::ErrorKind::InvalidData, "encoded track length overflow")
            })?;
            let source = track.encoded_content.clone();
            let offset = track.source_offset;
            let handle = ReaderHandle::new(
                format!("cue-bin:{}:{offset}:{size}", track.source),
                move || Ok(Box::new(RangeReader::new(source.open()?, offset, size)?)),
            );
            members.push(MaterializedNamedArtifact::new(
                bin_name,
                MaterializedArtifact::ReaderBacked { handle, size },
            ));
        }

        members.insert(
            0,
            MaterializedNamedArtifact::new(
                format!("{artifact_name}.cue"),
                MaterializedArtifact::Inline(cue.into_bytes()),
            ),
        );
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

fn validate_track(track: &CdTrack) -> io::Result<()> {
    if track.subchannel_content.is_some() {
        return Err(io::Error::new(
            io::ErrorKind::Unsupported,
            format!(
                "CD track {:02} contains subchannel data that CUE/BIN output cannot preserve",
                track.number
            ),
        ));
    }
    let index_one = track.index_one_sector().ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::InvalidData,
            format!("CD track {:02} has no INDEX 01", track.number),
        )
    })?;
    if index_one != track.file_backed_pregap_sectors {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!(
                "CD track {:02} INDEX 01 does not match its file-backed pregap",
                track.number
            ),
        ));
    }
    if track
        .indexes
        .iter()
        .any(|index| index.sector >= track.sector_count)
    {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!(
                "CD track {:02} has an index outside its extent",
                track.number
            ),
        ));
    }
    Ok(())
}

fn cue_track_mode(format: CdSectorFormat) -> &'static str {
    match format {
        CdSectorFormat::Mode1_2048 => "MODE1/2048",
        CdSectorFormat::Mode1_2352 => "MODE1/2352",
        CdSectorFormat::Mode2_2352 => "MODE2/2352",
        CdSectorFormat::Audio2352 => "AUDIO",
    }
}

fn cue_time(sectors: u64) -> io::Result<String> {
    let minutes = sectors / (75 * 60);
    if minutes > 99 {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "CUE time exceeds two-digit minute range",
        ));
    }
    let remainder = sectors % (75 * 60);
    Ok(format!(
        "{minutes:02}:{:02}:{:02}",
        remainder / 75,
        remainder % 75
    ))
}
