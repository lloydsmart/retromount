use std::collections::BTreeSet;

use crate::core::content::{DiscMedia, Platform};
use crate::output::capabilities::{CapabilityFeature, ContentType, Format};

/// Declarative presentation specifications.
///
/// A `PresentationSpec` describes desired output structure and artifact
/// requirements as data. Specs are later compiled into `PresentationPlan`.
///
/// This initial model is intentionally minimal and will be extended only as
/// real presentation use cases require it.
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

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LayoutSpec {
    Flat,
    GroupedByPlatformAndGame,
    LiteralRoot(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FileRuleSpec {
    pub directory: Vec<String>,
    pub select: SelectSpec,
    pub naming: NamingSpec,
    pub artifact: ArtifactSpec,
}

impl FileRuleSpec {
    pub fn new(select: SelectSpec, naming: NamingSpec, artifact: ArtifactSpec) -> Self {
        Self {
            directory: Vec::new(),
            select,
            naming,
            artifact,
        }
    }

    pub fn in_directory(mut self, directory: impl IntoIterator<Item = impl Into<String>>) -> Self {
        self.directory = directory.into_iter().map(Into::into).collect();
        self
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SelectSpec {
    /// Match any normalized game.
    Games,

    /// Match games that contain no parts.
    GamesWithoutParts,

    /// Match games with exactly one disc part.
    SingleDiscGames,

    /// Match a single-disc game for a specific platform.
    SingleDiscGamesByPlatform { platform: Platform },

    /// Match a single-disc game for a specific platform and media kind.
    SingleDiscGamesByPlatformAndMedia {
        platform: Platform,
        media: DiscMedia,
    },

    /// Match games consisting entirely of multiple disc parts.
    MultiDiscGames,

    /// Match multi-disc games for a specific platform.
    MultiDiscGamesByPlatform { platform: Platform },

    /// Match games with exactly one ROM part.
    SingleRomGames,

    /// Match normalized bytes content.
    Bytes,

    /// Match normalized text content.
    Text,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NamingSpec {
    /// Use the game title.
    GameTitle,

    /// Use the policy-derived game name.
    GameName,

    /// Use the policy-derived part name for a single-part game.
    PartName,

    /// Use the policy-derived playlist name for a game.
    PlaylistName,

    /// Use a source-derived/content-derived name.
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
                NamingSpec::PartName,
                ArtifactSpec::new(ContentType::Disc).with_format(Format::Bin),
            )],
        );

        assert_eq!(spec.layout, LayoutSpec::Flat);
        assert_eq!(spec.files.len(), 1);
        assert!(spec.files[0].directory.is_empty());
        assert_eq!(spec.files[0].select, SelectSpec::SingleDiscGames);
        assert_eq!(spec.files[0].naming, NamingSpec::PartName);
        assert_eq!(spec.files[0].artifact.content_type, ContentType::Disc);
        assert_eq!(spec.files[0].artifact.format, Some(Format::Bin));
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

    #[test]
    fn supports_rule_destination_directories() {
        let rule = FileRuleSpec::new(
            SelectSpec::Text,
            NamingSpec::Literal("readme.txt".to_string()),
            ArtifactSpec::new(ContentType::Text).with_format(Format::Text),
        )
        .in_directory(["docs", "manual"]);

        assert_eq!(rule.directory, ["docs", "manual"]);
    }
}
