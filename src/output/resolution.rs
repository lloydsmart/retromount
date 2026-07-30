use std::cmp::Reverse;

use crate::output::capabilities::{
    CapabilityFeature, CapabilityRequirements, ContentType, EncoderCapability, Format,
};
use crate::output::plan::{ArtifactRequest, PlannedArtifactKind};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RejectionReason {
    ContentTypeMismatch {
        required: ContentType,
        actual: ContentType,
    },
    FormatMismatch {
        required: Format,
        supported: Vec<Format>,
    },
    MissingRequiredFeature(CapabilityFeature),
    ForbiddenFeaturePresent(CapabilityFeature),
    MultiSourceNotSupported,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolutionScore {
    pub matched_preferred_features: usize,
    pub capability_priority: u32,
    pub capability_feature_count: usize,
}

impl ResolutionScore {
    pub fn new(
        matched_preferred_features: usize,
        capability_priority: u32,
        capability_feature_count: usize,
    ) -> Self {
        Self {
            matched_preferred_features,
            capability_priority,
            capability_feature_count,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CandidateDiagnostic {
    pub plugin_id: String,
    pub capability_id: String,
    pub is_accepted: bool,
    pub score: Option<ResolutionScore>,
    pub rejection_reasons: Vec<RejectionReason>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolutionDiagnostic {
    pub artifact_id: String,
    pub selected_capability_id: Option<String>,
    pub candidates: Vec<CandidateDiagnostic>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolvedCapability {
    pub plugin_id: String,
    pub capability_id: String,
    pub score: ResolutionScore,
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
        let mut accepted: Vec<ResolvedCapability> = Vec::new();
        let mut diagnostics = Vec::with_capacity(capabilities.len());

        for capability in capabilities {
            let rejection_reasons = self.rejection_reasons(request, capability);
            let is_accepted = rejection_reasons.is_empty();

            if is_accepted {
                let score = self.score(&request.requirements, capability);

                diagnostics.push(CandidateDiagnostic {
                    plugin_id: capability.plugin_id.clone(),
                    capability_id: capability.capability_id.clone(),
                    is_accepted: true,
                    score: Some(score.clone()),
                    rejection_reasons: Vec::new(),
                });

                accepted.push(ResolvedCapability {
                    plugin_id: capability.plugin_id.clone(),
                    capability_id: capability.capability_id.clone(),
                    score,
                });
            } else {
                diagnostics.push(CandidateDiagnostic {
                    plugin_id: capability.plugin_id.clone(),
                    capability_id: capability.capability_id.clone(),
                    is_accepted: false,
                    score: None,
                    rejection_reasons,
                });
            }
        }

        if accepted.is_empty() {
            return ResolutionResult::Unresolved {
                diagnostics: ResolutionDiagnostic {
                    artifact_id: request.id.0.clone(),
                    selected_capability_id: None,
                    candidates: diagnostics,
                },
            };
        }

        accepted.sort_by_key(|candidate| {
            (
                Reverse(candidate.score.matched_preferred_features),
                Reverse(candidate.score.capability_priority),
                candidate.score.capability_feature_count,
                candidate.plugin_id.clone(),
                candidate.capability_id.clone(),
            )
        });

        let selected = accepted.remove(0);

        ResolutionResult::Resolved {
            diagnostics: ResolutionDiagnostic {
                artifact_id: request.id.0.clone(),
                selected_capability_id: Some(selected.capability_id.clone()),
                candidates: diagnostics,
            },
            selected,
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
                reasons.push(RejectionReason::FormatMismatch {
                    required: format,
                    supported: capability.formats.iter().copied().collect(),
                });
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
            PlannedArtifactKind::ContentBacked(_) => {}
        }

        reasons
    }

    fn score(
        &self,
        requirements: &CapabilityRequirements,
        capability: &EncoderCapability,
    ) -> ResolutionScore {
        ResolutionScore::new(
            matched_preferred_features(requirements, capability),
            capability.priority,
            capability.features.len(),
        )
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
                assert_eq!(selected.score, ResolutionScore::new(0, 0, 0));
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
                assert_eq!(selected.score.matched_preferred_features, 2);
                assert_eq!(selected.score.capability_priority, 0);
                assert_eq!(selected.score.capability_feature_count, 2);
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
                assert_eq!(diagnostics.selected_capability_id, None);
                assert_eq!(diagnostics.candidates.len(), 1);
                assert_eq!(diagnostics.candidates[0].score, None);
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
                assert_eq!(diagnostics.selected_capability_id, None);
                assert_eq!(diagnostics.candidates[0].score, None);
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
                assert_eq!(selected.score.capability_priority, 20);
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

    #[test]
    fn diagnostics_capture_scores_for_accepted_candidates() {
        let resolver = CapabilityResolver;
        let request = request(
            CapabilityRequirements::new(ContentType::Disc)
                .with_format(Format::Chd)
                .prefer_feature(CapabilityFeature::Lossless),
        );

        let capabilities = vec![
            EncoderCapability::new("plugin.a", "cap.a", ContentType::Disc)
                .supports_format(Format::Chd)
                .with_feature(CapabilityFeature::Lossless)
                .with_priority(5),
            EncoderCapability::new("plugin.b", "cap.b", ContentType::Disc)
                .supports_format(Format::Zip),
        ];

        let result = resolver.resolve(&request, &capabilities);

        match result {
            ResolutionResult::Resolved { diagnostics, .. } => {
                assert_eq!(diagnostics.selected_capability_id.as_deref(), Some("cap.a"));
                assert_eq!(diagnostics.candidates.len(), 2);

                let accepted = &diagnostics.candidates[0];
                let rejected = &diagnostics.candidates[1];

                assert!(accepted.is_accepted);
                assert_eq!(accepted.score, Some(ResolutionScore::new(1, 5, 1)));

                assert!(!rejected.is_accepted);
                assert_eq!(rejected.score, None);
                assert_eq!(
                    rejected.rejection_reasons,
                    vec![RejectionReason::FormatMismatch {
                        required: Format::Chd,
                        supported: vec![Format::Zip],
                    }]
                );
            }
            ResolutionResult::Unresolved { .. } => panic!("expected resolution success"),
        }
    }

    #[test]
    fn prefers_more_specific_capability_when_other_scores_match() {
        let resolver = CapabilityResolver;
        let request =
            request(CapabilityRequirements::new(ContentType::Disc).with_format(Format::Chd));

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
                assert_eq!(selected.plugin_id, "plugin.a");
                assert_eq!(selected.capability_id, "cap.a");
                assert_eq!(selected.score.capability_feature_count, 1);
            }
            ResolutionResult::Unresolved { .. } => panic!("expected resolution success"),
        }
    }
}
