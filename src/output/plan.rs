use crate::core::content::LogicalDisc;
use crate::core::source::SourceRef;
use crate::output::capabilities::CapabilityRequirements;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PresentationPlan {
    pub entries: Vec<PlanEntry>,
}

impl PresentationPlan {
    pub fn new(entries: Vec<PlanEntry>) -> Self {
        Self { entries }
    }

    pub fn empty() -> Self {
        Self {
            entries: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PlanEntry {
    Directory(PlanDirectory),
    File(PlanFile),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlanDirectory {
    pub name: String,
    pub entries: Vec<PlanEntry>,
}

impl PlanDirectory {
    pub fn new(name: impl Into<String>, entries: Vec<PlanEntry>) -> Self {
        Self {
            name: name.into(),
            entries,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlanFile {
    pub name: String,
    pub artifact: ArtifactRequest,
}

impl PlanFile {
    pub fn new(name: impl Into<String>, artifact: ArtifactRequest) -> Self {
        Self {
            name: name.into(),
            artifact,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ArtifactId(pub String);

impl ArtifactId {
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ArtifactRequest {
    pub id: ArtifactId,
    pub kind: PlannedArtifactKind,
    pub requirements: CapabilityRequirements,
}

impl ArtifactRequest {
    pub fn new(
        id: ArtifactId,
        kind: PlannedArtifactKind,
        requirements: CapabilityRequirements,
    ) -> Self {
        Self {
            id,
            kind,
            requirements,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PlannedArtifactKind {
    SourceBacked(SourceArtifact),
    ContentBacked(ContentArtifact),
    Generated(GeneratedArtifact),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ContentArtifact {
    pub logical_disc: LogicalDisc,
}

impl ContentArtifact {
    pub fn logical_disc(logical_disc: LogicalDisc) -> Self {
        Self { logical_disc }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SourceArtifact {
    pub inputs: Vec<SourceArtifactInput>,
}

impl SourceArtifact {
    pub fn single(source: SourceRef, size: u64) -> Self {
        Self {
            inputs: vec![SourceArtifactInput { source, size }],
        }
    }

    pub fn multiple(inputs: Vec<SourceArtifactInput>) -> Self {
        Self { inputs }
    }

    pub fn is_multi_source(&self) -> bool {
        self.inputs.len() > 1
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SourceArtifactInput {
    pub source: SourceRef,
    pub size: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GeneratedArtifact {
    Playlist(PlaylistArtifact),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlaylistArtifact {
    pub entries: Vec<ArtifactReference>,
}

impl PlaylistArtifact {
    pub fn new(entries: Vec<ArtifactReference>) -> Self {
        Self { entries }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ArtifactReference {
    pub artifact_id: ArtifactId,
}

impl ArtifactReference {
    pub fn new(artifact_id: ArtifactId) -> Self {
        Self { artifact_id }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn source_artifact_single_is_not_multi_source() {
        let artifact = SourceArtifact::single(SourceRef::new("file:/roms/game.bin"), 1024);
        assert!(!artifact.is_multi_source());
        assert_eq!(artifact.inputs.len(), 1);
    }

    #[test]
    fn source_artifact_multiple_is_multi_source() {
        let artifact = SourceArtifact::multiple(vec![
            SourceArtifactInput {
                source: SourceRef::new("file:/roms/disc1.bin"),
                size: 1024,
            },
            SourceArtifactInput {
                source: SourceRef::new("file:/roms/disc2.bin"),
                size: 2048,
            },
        ]);

        assert!(artifact.is_multi_source());
        assert_eq!(artifact.inputs.len(), 2);
    }

    #[test]
    fn presentation_plan_empty_constructor_returns_no_entries() {
        let plan = PresentationPlan::empty();
        assert!(plan.entries.is_empty());
    }
}
