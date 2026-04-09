use crate::core::content::NormalizedContent;
use crate::output::plan::PresentationPlan;
use crate::policy::PolicySet;

/// Defines how normalized content is described for output.
///
/// Presenters are responsible for output structure, grouping, and layout.
/// They produce a declarative presentation plan describing what artifacts
/// should exist, but do not choose encoders or materialise output.
pub trait OutputPresenter: Send + Sync {
    fn present(&self, content: &[NormalizedContent], policy: &PolicySet) -> PresentationPlan;
}
