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
pub enum SourceRefSet {
    Single(SourceRef),
    Multiple(Vec<SourceRef>),
}

impl SourceRefSet {
    pub fn single(source: SourceRef) -> Self {
        Self::Single(source)
    }

    pub fn multiple(sources: Vec<SourceRef>) -> Self {
        Self::Multiple(sources)
    }

    pub fn len(&self) -> usize {
        match self {
            Self::Single(_) => 1,
            Self::Multiple(values) => values.len(),
        }
    }

    pub fn is_empty(&self) -> bool {
        match self {
            Self::Single(_) => false,
            Self::Multiple(values) => values.is_empty(),
        }
    }

    pub fn is_multi_source(&self) -> bool {
        self.len() > 1
    }

    pub fn iter(&self) -> Box<dyn Iterator<Item = &SourceRef> + '_> {
        match self {
            Self::Single(value) => Box::new(std::iter::once(value)),
            Self::Multiple(values) => Box::new(values.iter()),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ArtifactRequest {
    pub id: ArtifactId,
    pub sources: SourceRefSet,
    pub requirements: CapabilityRequirements,
}

impl ArtifactRequest {
    pub fn new(
        id: ArtifactId,
        sources: SourceRefSet,
        requirements: CapabilityRequirements,
    ) -> Self {
        Self {
            id,
            sources,
            requirements,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn source_ref_set_single_is_not_multi_source() {
        let sources = SourceRefSet::single(SourceRef::new("file:/roms/game.bin"));
        assert_eq!(sources.len(), 1);
        assert!(!sources.is_multi_source());
        assert!(!sources.is_empty());
    }

    #[test]
    fn source_ref_set_multiple_is_multi_source() {
        let sources = SourceRefSet::multiple(vec![
            SourceRef::new("file:/roms/disc1.bin"),
            SourceRef::new("file:/roms/disc2.bin"),
        ]);

        assert_eq!(sources.len(), 2);
        assert!(sources.is_multi_source());
        assert!(!sources.is_empty());
    }

    #[test]
    fn presentation_plan_empty_constructor_returns_no_entries() {
        let plan = PresentationPlan::empty();
        assert!(plan.entries.is_empty());
    }
}
