#![cfg(unix)]

use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};

use retromount::core::content::{GamePart, NormalizedContent};
use retromount::core::vfs::VfsNode;
use retromount::engine::components::default_pipeline_components;
use retromount::engine::pipeline::{run_pipeline_with_options, PipelineOptions};
use retromount::engine::plugins::load_plugin_registry;
use retromount::input::file_source::FileInputSource;
use retromount::output::capabilities::{CapabilityRequirements, ContentType, Format};
use retromount::output::plan::{
    ArtifactId, ArtifactRequest, PlanEntry, PlanFile, PlannedArtifactKind, PresentationPlan,
    SourceArtifact,
};
use retromount::output::present::OutputPresenter;
use retromount::policy::PolicySet;
use tempfile::TempDir;

struct PluginOnlyDiscPresenter;

impl OutputPresenter for PluginOnlyDiscPresenter {
    fn present(&self, content: &[NormalizedContent], _policy: &PolicySet) -> PresentationPlan {
        let disc_source = content
            .iter()
            .find_map(|item| match item {
                NormalizedContent::Game(game) => game.parts.iter().find_map(|part| match part {
                    GamePart::Disc(disc) => Some(disc.source.clone()),
                    _ => None,
                }),
                _ => None,
            })
            .expect("expected at least one normalized disc game");

        PresentationPlan::new(vec![PlanEntry::File(PlanFile::new(
            "Plugin Test.chd",
            ArtifactRequest::new(
                ArtifactId::new("plugin-test-disc"),
                PlannedArtifactKind::SourceBacked(SourceArtifact::single(disc_source, 0)),
                CapabilityRequirements::new(ContentType::Disc).with_format(Format::Chd),
            ),
        ))])
    }
}

fn write_test_disc(dir: &Path) -> PathBuf {
    let bin_path = dir.join("Crash Bandicoot.bin");
    let cue_path = dir.join("Crash Bandicoot.cue");

    fs::write(&bin_path, b"fake-disc-bytes").unwrap();
    fs::write(
        &cue_path,
        concat!(
            "FILE \"Crash Bandicoot.bin\" BINARY\n",
            "  TRACK 01 MODE2/2352\n",
            "    INDEX 01 00:00:00\n",
        ),
    )
    .unwrap();

    cue_path
}

fn copy_fixture_plugin_to_tempdir(dir: &Path) -> PathBuf {
    let source = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("plugins")
        .join("fixtures")
        .join("test-inline-encoder.sh");

    assert!(
        source.exists(),
        "fixture plugin not found at {}",
        source.display()
    );

    let destination = dir.join("test-inline-encoder.sh");
    fs::copy(&source, &destination).unwrap();

    let mut permissions = fs::metadata(&destination).unwrap().permissions();
    permissions.set_mode(0o755);
    fs::set_permissions(&destination, permissions).unwrap();

    destination
}

#[test]
fn pipeline_materializes_disc_via_fixture_plugin() {
    let temp = TempDir::new().unwrap();

    let input_dir = temp.path().join("input");
    let plugin_dir = temp.path().join("plugins");
    fs::create_dir_all(&input_dir).unwrap();
    fs::create_dir_all(&plugin_dir).unwrap();

    let cue_path = write_test_disc(&input_dir);
    let _plugin_path = copy_fixture_plugin_to_tempdir(&plugin_dir);

    let plugin_registry = load_plugin_registry(&plugin_dir).unwrap();
    assert!(
        !plugin_registry.is_empty(),
        "expected fixture plugin registry to contain at least one plugin"
    );

    let source = FileInputSource::new(&cue_path);
    let components = default_pipeline_components().unwrap();
    let presenter = PluginOnlyDiscPresenter;

    let trace = run_pipeline_with_options(
        &source,
        components.identifier.as_ref(),
        components.decoder.as_ref(),
        &presenter,
        &components.policy,
        &PipelineOptions {
            normalization: Default::default(),
            plugin_registry: Some(&plugin_registry),
        },
    )
    .unwrap();

    assert_eq!(trace.output_vfs.children().len(), 1);

    let file = match &trace.output_vfs.children()[0] {
        VfsNode::File(file) => file,
        other => panic!("expected file at root, got {other:?}"),
    };

    assert_eq!(file.name, "Plugin Test.chd");

    match &file.backing {
        retromount::core::vfs::FileBacking::Inline(bytes) => {
            assert_eq!(bytes, b"PLUGIN");
            assert_eq!(file.size, 6);
        }
        other => panic!("expected inline-backed file from fixture plugin, got {other:?}"),
    }
}
