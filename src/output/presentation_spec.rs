use std::collections::BTreeSet;

use crate::output::capabilities::{CapabilityFeature, ContentType, Format};

/// Declarative presentation specifications.
///
/// A `PresentationSpec` describes desired output structure and artifact
/// requirements as data. Specs are later compiled into `PresentationPlan`.
///
/// This initial model is intentionally minimal:
///
/// - one top-level layout choice
/// - a flat list of file rules
/// - simple content selection
/// - simple naming
/// - artifact requirements that align with encoder capability resolution
///
/// It is expected to evolve incrementally as real presentation use cases
/// require more expressive power.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PresentationSpec {
    pub layout: LayoutSpec,
    pub files: Vec<FileRuleSpec>,
}

impl PresentationSpec {
    pub fn new(layout: LayoutSpec, files: Vec<FileRuleSpec>) -> Self {
        Self { layout, files }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LayoutSpec {
    Flat,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FileRuleSpec {
    pub select: SelectSpec,
    pub naming: NamingSpec,
    pub artifact: ArtifactSpec,
}

impl FileRuleSpec {
    pub fn new(select: SelectSpec, naming: NamingSpec, artifact: ArtifactSpec) -> Self {
        Self {
            select,
            naming,
            artifact,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SelectSpec {
    /// Match any normalized game.
    ///
    /// This is deliberately broad and may later be refined or replaced by a
    /// richer matching model once compilation needs make that necessary.
    Games,

    /// Match games with exactly one disc part.
    SingleDiscGames,

    /// Match games with exactly one ROM part.
    SingleRomGames,

    /// Match normalized bytes content.
    Bytes,

    /// Match normalized text content.
    Text,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NamingSpec {
    /// Use the game title, with any file extension supplied later by
    /// compilation/materialization rules.
    GameTitle,
    /// Use the source-derived or content-derived name.
    /// The exact resolution of this name is deferred to the compiler.
    PartName,
    SourceName,
    /// Use a fixed literal file name.
    Literal(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ArtifactSpec {
    pub content_type: ContentType,
    pub format: Option<Format>,
    pub required_features: BTreeSet<CapabilityFeature>,
    pub preferred_features: BTreeSet<CapabilityFeature>,
    pub forbidden_features: BTreeSet<CapabilityFeature>,
}

impl ArtifactSpec {
    pub fn new(content_type: ContentType) -> Self {
        Self {
            content_type,
            format: None,
            required_features: BTreeSet::new(),
            preferred_features: BTreeSet::new(),
            forbidden_features: BTreeSet::new(),
        }
    }

    pub fn with_format(mut self, format: Format) -> Self {
        self.format = Some(format);
        self
    }

    pub fn require_feature(mut self, feature: CapabilityFeature) -> Self {
        self.required_features.insert(feature);
        self
    }

    pub fn prefer_feature(mut self, feature: CapabilityFeature) -> Self {
        self.preferred_features.insert(feature);
        self
    }

    pub fn forbid_feature(mut self, feature: CapabilityFeature) -> Self {
        self.forbidden_features.insert(feature);
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builds_minimal_flat_disc_spec() {
        let spec = PresentationSpec::new(
            LayoutSpec::Flat,
            vec![FileRuleSpec::new(
                SelectSpec::SingleDiscGames,
                NamingSpec::GameTitle,
                ArtifactSpec::new(ContentType::Disc).with_format(Format::Iso),
            )],
        );

        assert_eq!(spec.layout, LayoutSpec::Flat);
        assert_eq!(spec.files.len(), 1);
        assert_eq!(spec.files[0].select, SelectSpec::SingleDiscGames);
        assert_eq!(spec.files[0].naming, NamingSpec::GameTitle);
        assert_eq!(spec.files[0].artifact.content_type, ContentType::Disc);
        assert_eq!(spec.files[0].artifact.format, Some(Format::Iso));
    }

    #[test]
    fn artifact_spec_builder_sets_feature_sets() {
        let artifact = ArtifactSpec::new(ContentType::Disc)
            .with_format(Format::Chd)
            .require_feature(CapabilityFeature::Lossless)
            .prefer_feature(CapabilityFeature::RandomAccess)
            .forbid_feature(CapabilityFeature::SupportsPartial);

        assert_eq!(artifact.content_type, ContentType::Disc);
        assert_eq!(artifact.format, Some(Format::Chd));
        assert!(artifact
            .required_features
            .contains(&CapabilityFeature::Lossless));
        assert!(artifact
            .preferred_features
            .contains(&CapabilityFeature::RandomAccess));
        assert!(artifact
            .forbidden_features
            .contains(&CapabilityFeature::SupportsPartial));
    }

    #[test]
    fn supports_literal_names() {
        let rule = FileRuleSpec::new(
            SelectSpec::Text,
            NamingSpec::Literal("readme.txt".to_string()),
            ArtifactSpec::new(ContentType::Text).with_format(Format::Text),
        );

        assert_eq!(rule.select, SelectSpec::Text);
        assert_eq!(rule.naming, NamingSpec::Literal("readme.txt".to_string()));
        assert_eq!(rule.artifact.content_type, ContentType::Text);
        assert_eq!(rule.artifact.format, Some(Format::Text));
    }
}
