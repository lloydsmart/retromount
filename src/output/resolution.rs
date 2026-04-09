use std::cmp::Reverse;

use crate::output::capabilities::{
    CapabilityFeature, CapabilityRequirements, ContentType, EncoderCapability,
};
use crate::output::plan::{ArtifactRequest, PlannedArtifactKind};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RejectionReason {
    ContentTypeMismatch {
        required: ContentType,
        actual: ContentType,
    },
    FormatMismatch,
    MissingRequiredFeature(CapabilityFeature),
    ForbiddenFeaturePresent(CapabilityFeature),
    MultiSourceNotSupported,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CandidateDiagnostic {
    pub plugin_id: String,
    pub capability_id: String,
    pub accepted: bool,
    pub matched_preferred_features: usize,
    pub rejection_reasons: Vec<RejectionReason>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolutionDiagnostic {
    pub artifact_id: String,
    pub candidates: Vec<CandidateDiagnostic>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolvedCapability {
    pub plugin_id: String,
    pub capability_id: String,
    pub matched_preferred_features: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ResolutionResult {
    Resolved {
        selected: ResolvedCapability,
        diagnostics: ResolutionDiagnostic,
    },
    Unresolved {
        diagnostics: ResolutionDiagnostic,
    },
}

#[derive(Debug, Default)]
pub struct CapabilityResolver;

impl CapabilityResolver {
    pub fn resolve(
        &self,
        request: &ArtifactRequest,
        capabilities: &[EncoderCapability],
    ) -> ResolutionResult {
        let mut accepted: Vec<(ResolvedCapability, u32, usize)> = Vec::new();
        let mut diagnostics = Vec::with_capacity(capabilities.len());

        for capability in capabilities {
            let rejection_reasons = self.rejection_reasons(request, capability);
            let matched_preferred_features =
                matched_preferred_features(&request.requirements, capability);

            let accepted_candidate = rejection_reasons.is_empty();

            diagnostics.push(CandidateDiagnostic {
                plugin_id: capability.plugin_id.clone(),
                capability_id: capability.capability_id.clone(),
                accepted: accepted_candidate,
                matched_preferred_features,
                rejection_reasons,
            });

            if accepted_candidate {
                accepted.push((
                    ResolvedCapability {
                        plugin_id: capability.plugin_id.clone(),
                        capability_id: capability.capability_id.clone(),
                        matched_preferred_features,
                    },
                    capability.priority,
                    capability.features.len(),
                ));
            }
        }

        let diagnostic_bundle = ResolutionDiagnostic {
            artifact_id: request.id.0.clone(),
            candidates: diagnostics,
        };

        if accepted.is_empty() {
            return ResolutionResult::Unresolved {
                diagnostics: diagnostic_bundle,
            };
        }

        accepted.sort_by_key(|(selected, priority, feature_count)| {
            (
                Reverse(selected.matched_preferred_features),
                Reverse(*priority),
                Reverse(*feature_count),
                selected.plugin_id.clone(),
                selected.capability_id.clone(),
            )
        });

        let (selected, _, _) = accepted.remove(0);

        ResolutionResult::Resolved {
            selected,
            diagnostics: diagnostic_bundle,
        }
    }

    fn rejection_reasons(
        &self,
        request: &ArtifactRequest,
        capability: &EncoderCapability,
    ) -> Vec<RejectionReason> {
        let mut reasons = Vec::new();
        let requirements: &CapabilityRequirements = &request.requirements;

        if capability.content_type != requirements.content_type {
            reasons.push(RejectionReason::ContentTypeMismatch {
                required: requirements.content_type,
                actual: capability.content_type,
            });
        }

        if let Some(format) = requirements.format {
            if !capability.formats.contains(&format) {
                reasons.push(RejectionReason::FormatMismatch);
            }
        }

        for feature in &requirements.required_features {
            if !capability.features.contains(feature) {
                reasons.push(RejectionReason::MissingRequiredFeature(*feature));
            }
        }

        for feature in &requirements.forbidden_features {
            if capability.features.contains(feature) {
                reasons.push(RejectionReason::ForbiddenFeaturePresent(*feature));
            }
        }

        match &request.kind {
            PlannedArtifactKind::SourceBacked(source_artifact) => {
                if source_artifact.is_multi_source() && !capability.supports_multi_source() {
                    reasons.push(RejectionReason::MultiSourceNotSupported);
                }
            }
            PlannedArtifactKind::Generated(_) => {}
        }

        reasons
    }
}

fn matched_preferred_features(
    requirements: &CapabilityRequirements,
    capability: &EncoderCapability,
) -> usize {
    requirements
        .preferred_features
        .iter()
        .filter(|feature| capability.features.contains(feature))
        .count()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::source::SourceRef;
    use crate::output::capabilities::{CapabilityFeature, Format};
    use crate::output::plan::{
        ArtifactId, PlannedArtifactKind, SourceArtifact, SourceArtifactInput,
    };

    fn request(requirements: CapabilityRequirements) -> ArtifactRequest {
        ArtifactRequest {
            id: ArtifactId::new("artifact-1"),
            kind: PlannedArtifactKind::SourceBacked(SourceArtifact::single(
                SourceRef::new("file:/roms/game.bin"),
                1024,
            )),
            requirements,
        }
    }

    #[test]
    fn resolves_exact_match() {
        let resolver = CapabilityResolver;
        let request =
            request(CapabilityRequirements::new(ContentType::Disc).with_format(Format::Chd));

        let capabilities = vec![
            EncoderCapability::new("builtin.zip", "disc.zip", ContentType::Disc)
                .supports_format(Format::Zip),
            EncoderCapability::new("builtin.chd", "disc.chd", ContentType::Disc)
                .supports_format(Format::Chd),
        ];

        let result = resolver.resolve(&request, &capabilities);

        match result {
            ResolutionResult::Resolved { selected, .. } => {
                assert_eq!(selected.plugin_id, "builtin.chd");
                assert_eq!(selected.capability_id, "disc.chd");
            }
            ResolutionResult::Unresolved { .. } => panic!("expected resolution success"),
        }
    }

    #[test]
    fn prefers_more_preferred_features() {
        let resolver = CapabilityResolver;
        let request = request(
            CapabilityRequirements::new(ContentType::Disc)
                .with_format(Format::Chd)
                .prefer_feature(CapabilityFeature::Lossless)
                .prefer_feature(CapabilityFeature::RandomAccess),
        );

        let capabilities = vec![
            EncoderCapability::new("plugin.a", "cap.a", ContentType::Disc)
                .supports_format(Format::Chd)
                .with_feature(CapabilityFeature::Lossless),
            EncoderCapability::new("plugin.b", "cap.b", ContentType::Disc)
                .supports_format(Format::Chd)
                .with_feature(CapabilityFeature::Lossless)
                .with_feature(CapabilityFeature::RandomAccess),
        ];

        let result = resolver.resolve(&request, &capabilities);

        match result {
            ResolutionResult::Resolved { selected, .. } => {
                assert_eq!(selected.plugin_id, "plugin.b");
                assert_eq!(selected.capability_id, "cap.b");
                assert_eq!(selected.matched_preferred_features, 2);
            }
            ResolutionResult::Unresolved { .. } => panic!("expected resolution success"),
        }
    }

    #[test]
    fn rejects_forbidden_feature() {
        let resolver = CapabilityResolver;
        let request = request(
            CapabilityRequirements::new(ContentType::Disc)
                .with_format(Format::Chd)
                .forbid_feature(CapabilityFeature::SupportsPartial),
        );

        let capabilities = vec![
            EncoderCapability::new("plugin.a", "cap.a", ContentType::Disc)
                .supports_format(Format::Chd)
                .with_feature(CapabilityFeature::SupportsPartial),
        ];

        let result = resolver.resolve(&request, &capabilities);

        match result {
            ResolutionResult::Resolved { .. } => panic!("expected unresolved result"),
            ResolutionResult::Unresolved { diagnostics } => {
                assert_eq!(diagnostics.candidates.len(), 1);
                assert_eq!(
                    diagnostics.candidates[0].rejection_reasons,
                    vec![RejectionReason::ForbiddenFeaturePresent(
                        CapabilityFeature::SupportsPartial
                    )]
                );
            }
        }
    }

    #[test]
    fn rejects_multi_source_when_not_supported() {
        let resolver = CapabilityResolver;

        let request = ArtifactRequest {
            id: ArtifactId::new("playlist-1"),
            kind: PlannedArtifactKind::SourceBacked(SourceArtifact::multiple(vec![
                SourceArtifactInput {
                    source: SourceRef::new("file:/roms/disc1.chd"),
                    size: 1024,
                },
                SourceArtifactInput {
                    source: SourceRef::new("file:/roms/disc2.chd"),
                    size: 2048,
                },
            ])),
            requirements: CapabilityRequirements::new(ContentType::Playlist)
                .with_format(Format::M3u),
        };

        let capabilities = vec![
            EncoderCapability::new("plugin.a", "cap.a", ContentType::Playlist)
                .supports_format(Format::M3u),
        ];

        let result = resolver.resolve(&request, &capabilities);

        match result {
            ResolutionResult::Resolved { .. } => panic!("expected unresolved result"),
            ResolutionResult::Unresolved { diagnostics } => {
                assert_eq!(
                    diagnostics.candidates[0].rejection_reasons,
                    vec![RejectionReason::MultiSourceNotSupported]
                );
            }
        }
    }

    #[test]
    fn higher_priority_wins_when_scores_match() {
        let resolver = CapabilityResolver;
        let request =
            request(CapabilityRequirements::new(ContentType::Disc).with_format(Format::Chd));

        let capabilities = vec![
            EncoderCapability::new("plugin.a", "cap.a", ContentType::Disc)
                .supports_format(Format::Chd)
                .with_priority(10),
            EncoderCapability::new("plugin.b", "cap.b", ContentType::Disc)
                .supports_format(Format::Chd)
                .with_priority(20),
        ];

        let result = resolver.resolve(&request, &capabilities);

        match result {
            ResolutionResult::Resolved { selected, .. } => {
                assert_eq!(selected.plugin_id, "plugin.b");
                assert_eq!(selected.capability_id, "cap.b");
            }
            ResolutionResult::Unresolved { .. } => panic!("expected resolution success"),
        }
    }

    #[test]
    fn tie_breaks_deterministically() {
        let resolver = CapabilityResolver;
        let request =
            request(CapabilityRequirements::new(ContentType::Disc).with_format(Format::Chd));

        let capabilities = vec![
            EncoderCapability::new("plugin.z", "cap.z", ContentType::Disc)
                .supports_format(Format::Chd),
            EncoderCapability::new("plugin.a", "cap.a", ContentType::Disc)
                .supports_format(Format::Chd),
        ];

        let result = resolver.resolve(&request, &capabilities);

        match result {
            ResolutionResult::Resolved { selected, .. } => {
                assert_eq!(selected.plugin_id, "plugin.a");
                assert_eq!(selected.capability_id, "cap.a");
            }
            ResolutionResult::Unresolved { .. } => panic!("expected resolution success"),
        }
    }
}
