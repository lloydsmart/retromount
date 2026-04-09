use std::collections::BTreeSet;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ContentType {
    Rom,
    Disc,
    Playlist,
    Archive,
    Directory,
    Bytes,
    Text,
    Game,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Format {
    Iso,
    Chd,
    Zip,
    M3u,
    Directory,
    Bin,
    Text,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum CapabilityFeature {
    MultiSource,
    Streaming,
    Lossless,
    RandomAccess,
    SupportsPartial,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CapabilityRequirements {
    pub content_type: ContentType,
    pub format: Option<Format>,
    pub required_features: BTreeSet<CapabilityFeature>,
    pub preferred_features: BTreeSet<CapabilityFeature>,
    pub forbidden_features: BTreeSet<CapabilityFeature>,
}

impl CapabilityRequirements {
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

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EncoderCapability {
    pub plugin_id: String,
    pub capability_id: String,
    pub content_type: ContentType,
    pub formats: BTreeSet<Format>,
    pub features: BTreeSet<CapabilityFeature>,
    pub priority: u32,
}

impl EncoderCapability {
    pub fn new(
        plugin_id: impl Into<String>,
        capability_id: impl Into<String>,
        content_type: ContentType,
    ) -> Self {
        Self {
            plugin_id: plugin_id.into(),
            capability_id: capability_id.into(),
            content_type,
            formats: BTreeSet::new(),
            features: BTreeSet::new(),
            priority: 0,
        }
    }

    pub fn supports_format(mut self, format: Format) -> Self {
        self.formats.insert(format);
        self
    }

    pub fn with_feature(mut self, feature: CapabilityFeature) -> Self {
        self.features.insert(feature);
        self
    }

    pub fn with_priority(mut self, priority: u32) -> Self {
        self.priority = priority;
        self
    }

    pub fn supports_multi_source(&self) -> bool {
        self.features.contains(&CapabilityFeature::MultiSource)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn capability_requirements_builder_sets_fields() {
        let requirements = CapabilityRequirements::new(ContentType::Disc)
            .with_format(Format::Chd)
            .require_feature(CapabilityFeature::Lossless)
            .prefer_feature(CapabilityFeature::RandomAccess)
            .forbid_feature(CapabilityFeature::SupportsPartial);

        assert_eq!(requirements.content_type, ContentType::Disc);
        assert_eq!(requirements.format, Some(Format::Chd));
        assert!(requirements
            .required_features
            .contains(&CapabilityFeature::Lossless));
        assert!(requirements
            .preferred_features
            .contains(&CapabilityFeature::RandomAccess));
        assert!(requirements
            .forbidden_features
            .contains(&CapabilityFeature::SupportsPartial));
    }

    #[test]
    fn encoder_capability_builder_sets_fields() {
        let capability = EncoderCapability::new("builtin.chd", "disc.chd", ContentType::Disc)
            .supports_format(Format::Chd)
            .with_feature(CapabilityFeature::Lossless)
            .with_priority(100);

        assert!(capability.formats.contains(&Format::Chd));
        assert!(capability.features.contains(&CapabilityFeature::Lossless));
        assert_eq!(capability.priority, 100);
    }
}
