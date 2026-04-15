use std::collections::HashSet;
use std::io;

use crate::core::content::{ContentMeta, DecodedContent, NormalizedContent};
use crate::core::normalizer::{normalize_decoded_content, NormalizationOptions};
use crate::core::source::SourceObject;
use crate::core::vfs::VfsDirectory;
use crate::input::decode::InputDecoder;
use crate::input::identify::{InputIdentifier, InputIdentity};
use crate::input::source::InputSource;
use crate::output::materialize::{materialize_plan, materialize_plan_with_plugins};
use crate::output::plan::PresentationPlan;
use crate::output::plugin_registry::PluginRegistry;
use crate::output::present::OutputPresenter;
use crate::policy::PolicySet;
use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
pub struct PipelineTrace {
    pub objects: Vec<TracedObject>,
    pub normalized: Vec<NormalizedContent>,
    pub output_vfs: VfsDirectory,
}

#[derive(Debug, Clone, Serialize)]
pub struct TracedObject {
    pub object: SourceObject,
    pub identity: InputIdentity,
    pub supported: bool,
    pub decoded: Vec<DecodedContent>,
}

#[derive(Default)]
pub struct PipelineOptions<'a> {
    pub normalization: NormalizationOptions,
    pub plugin_registry: Option<&'a PluginRegistry>,
}

pub fn run_pipeline(
    source: &dyn InputSource,
    identifier: &dyn InputIdentifier,
    decoder: &dyn InputDecoder,
    presenter: &dyn OutputPresenter,
    policy: &PolicySet,
) -> Result<VfsDirectory, io::Error> {
    Ok(run_pipeline_with_options(
        source,
        identifier,
        decoder,
        presenter,
        policy,
        &PipelineOptions::default(),
    )?
    .output_vfs)
}

pub fn run_pipeline_with_trace(
    source: &dyn InputSource,
    identifier: &dyn InputIdentifier,
    decoder: &dyn InputDecoder,
    presenter: &dyn OutputPresenter,
    policy: &PolicySet,
) -> Result<PipelineTrace, io::Error> {
    run_pipeline_with_options(
        source,
        identifier,
        decoder,
        presenter,
        policy,
        &PipelineOptions::default(),
    )
}

pub fn run_pipeline_with_options(
    source: &dyn InputSource,
    identifier: &dyn InputIdentifier,
    decoder: &dyn InputDecoder,
    presenter: &dyn OutputPresenter,
    policy: &PolicySet,
    options: &PipelineOptions<'_>,
) -> Result<PipelineTrace, io::Error> {
    let objects = source.enumerate()?;
    let mut traced_objects = Vec::new();
    let mut all_decoded_content = Vec::new();

    for object in objects {
        let identity = identifier.identify(&object)?;
        let supported = decoder.supports(&identity);

        let decoded = if supported {
            decoder.decode(&object, &identity)?
        } else {
            Vec::new()
        };

        all_decoded_content.extend(decoded.iter().cloned());

        traced_objects.push(TracedObject {
            object,
            identity,
            supported,
            decoded,
        });
    }

    let normalized = normalize_decoded_content(all_decoded_content, &options.normalization);
    let normalized_presentable_content = suppress_consumed_content(&normalized);
    let presentation_plan = presenter.present(&normalized_presentable_content, policy);
    let output_vfs = materialize_presentation_plan(&presentation_plan, options)?;

    Ok(PipelineTrace {
        objects: traced_objects,
        normalized,
        output_vfs,
    })
}

fn materialize_presentation_plan(
    plan: &PresentationPlan,
    options: &PipelineOptions<'_>,
) -> Result<VfsDirectory, io::Error> {
    match options.plugin_registry {
        Some(plugin_registry) => materialize_plan_with_plugins(plan, plugin_registry),
        None => materialize_plan(plan),
    }
}

fn suppress_consumed_content(all_content: &[NormalizedContent]) -> Vec<NormalizedContent> {
    let consumed_sources: HashSet<_> = all_content
        .iter()
        .flat_map(|content| content.consumed_sources().iter().cloned())
        .collect();

    all_content
        .iter()
        .filter(|content| !consumed_sources.contains(content.source()))
        .cloned()
        .collect()
}
