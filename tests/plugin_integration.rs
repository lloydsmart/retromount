#![cfg(unix)]

use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};

use retromount::core::vfs::VfsNode;
use retromount::engine::components::default_pipeline_components;
use retromount::engine::mount::prepare_mount_session_from_presentation;
use retromount::engine::pipeline::{run_pipeline_with_presentation_options, PipelineOptions};
use retromount::engine::plugins::load_plugin_registry;
use retromount::input::file_source::FileInputSource;
use retromount::mount::session::MountNodeKind;
use retromount::output::capabilities::{ContentType, Format};
use retromount::output::presentation_spec::{
    ArtifactSpec, FileRuleSpec, LayoutSpec, NamingSpec, PresentationSpec, SelectSpec,
};
use tempfile::TempDir;

fn disc_presentation(name: &str, format: Format) -> PresentationSpec {
    PresentationSpec::new(
        LayoutSpec::Flat,
        vec![FileRuleSpec::new(
            SelectSpec::SingleDiscGames,
            NamingSpec::Literal(name.to_string()),
            ArtifactSpec::new(ContentType::Disc).with_format(format),
        )],
    )
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
    copy_named_fixture_plugin_to_tempdir("test-inline-encoder.sh", dir)
}

fn copy_named_fixture_plugin_to_tempdir(file_name: &str, dir: &Path) -> PathBuf {
    let source = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("plugins")
        .join("fixtures")
        .join(file_name);

    assert!(
        source.exists(),
        "fixture plugin not found at {}",
        source.display()
    );

    let destination = dir.join(file_name);
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
    let presentation = disc_presentation("Plugin Test.chd", Format::Chd);

    let trace = run_pipeline_with_presentation_options(
        &source,
        components.identifier.as_ref(),
        components.decoder.as_ref(),
        &presentation,
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

#[test]
fn mount_preparation_uses_runtime_plugin_materialization() {
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
    let presentation = disc_presentation("Plugin Test.chd", Format::Chd);

    let session = prepare_mount_session_from_presentation(
        &source,
        components.identifier.as_ref(),
        components.decoder.as_ref(),
        &presentation,
        &components.policy,
        Some(&plugin_registry),
    )
    .unwrap();

    let root_children = session
        .children(session.root_inode())
        .expect("expected root inode to have children");

    assert_eq!(root_children.len(), 1);

    let file_node = root_children[0];
    assert_eq!(file_node.name, "Plugin Test.chd");

    let file = match &file_node.kind {
        MountNodeKind::File { file } => file,
        other => panic!("expected mounted file node, got {other:?}"),
    };

    assert_eq!(file.name, "Plugin Test.chd");
    assert_eq!(file.size, 6);

    match &file.backing {
        retromount::core::vfs::FileBacking::Inline(bytes) => {
            assert_eq!(bytes, b"PLUGIN");
        }
        other => panic!("expected inline-backed file from fixture plugin, got {other:?}"),
    }
}

#[test]
fn mount_preparation_fails_when_no_encoder_matches() {
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
    let presentation = disc_presentation("Unmatched.m3u", Format::M3u);

    let error = prepare_mount_session_from_presentation(
        &source,
        components.identifier.as_ref(),
        components.decoder.as_ref(),
        &presentation,
        &components.policy,
        Some(&plugin_registry),
    )
    .unwrap_err();

    let message = error.to_string();

    assert!(
        message.contains("unmatched-disc") || message.contains("Unmatched.m3u"),
        "expected error to mention artifact identity, got: {message}"
    );
    assert!(
        message.contains("no encoder could materialize artifact")
            || message.contains("ResolutionDiagnostic")
            || message.contains("FormatMismatch")
            || message.contains("ContentTypeMismatch"),
        "expected error to describe encoder resolution failure, got: {message}"
    );
    assert!(
        !message.contains("Failed to open config file"),
        "expected mount preparation failure to preserve pipeline context, got: {message}"
    );
}

#[test]
fn mount_preparation_fails_when_selected_plugin_materialization_fails() {
    let temp = TempDir::new().unwrap();

    let input_dir = temp.path().join("input");
    let plugin_dir = temp.path().join("plugins");
    fs::create_dir_all(&input_dir).unwrap();
    fs::create_dir_all(&plugin_dir).unwrap();

    let cue_path = write_test_disc(&input_dir);
    let _plugin_path = copy_named_fixture_plugin_to_tempdir("test-failing-encoder.sh", &plugin_dir);

    let plugin_registry = load_plugin_registry(&plugin_dir).unwrap();
    assert!(
        !plugin_registry.is_empty(),
        "expected failing fixture plugin registry to contain at least one plugin"
    );

    let source = FileInputSource::new(&cue_path);
    let components = default_pipeline_components().unwrap();
    let presentation = disc_presentation("Plugin Test.chd", Format::Chd);

    let error = prepare_mount_session_from_presentation(
        &source,
        components.identifier.as_ref(),
        components.decoder.as_ref(),
        &presentation,
        &components.policy,
        Some(&plugin_registry),
    )
    .unwrap_err();

    let message = error.to_string();

    assert!(
        message.contains("Plugin Test.chd") || message.contains("plugin-test-disc"),
        "expected error to mention requested artifact, got: {message}"
    );
    assert!(
        message.contains("plugin.fixture.failing")
            || message.contains("fixture.disc.failing")
            || message.contains("test-failing-encoder.sh"),
        "expected error to mention selected plugin context, got: {message}"
    );
    assert!(
        message.contains("fixture plugin forced materialization failure")
            || message.contains("non-zero status 42"),
        "expected error to describe plugin runtime failure, got: {message}"
    );
    assert!(
        !message.contains("Failed to open config file"),
        "expected mount preparation failure to preserve runtime plugin context, got: {message}"
    );
}

#[test]
fn load_plugin_registry_fails_for_invalid_discovered_plugin() {
    let temp = TempDir::new().unwrap();
    let plugin_dir = temp.path().join("plugins");
    fs::create_dir_all(&plugin_dir).unwrap();

    let _plugin_path =
        copy_named_fixture_plugin_to_tempdir("test-invalid-manifest-encoder.sh", &plugin_dir);

    let error = match load_plugin_registry(&plugin_dir) {
        Ok(_) => panic!("expected invalid discovered plugin to fail registry loading"),
        Err(error) => error,
    };

    let message = error.to_string();

    assert!(
        message.contains("plugin")
            || message.contains("manifest")
            || message.contains("rejected")
            || message.contains("invalid"),
        "expected discovery/load failure context, got: {message}"
    );
    assert!(
        message.contains("test-invalid-manifest-encoder.sh")
            || message.contains("plugin_id")
            || message.contains("empty"),
        "expected invalid plugin details in error, got: {message}"
    );
}

#[test]
fn mount_preparation_fails_when_selected_plugin_returns_malformed_response() {
    let temp = TempDir::new().unwrap();

    let input_dir = temp.path().join("input");
    let plugin_dir = temp.path().join("plugins");
    fs::create_dir_all(&input_dir).unwrap();
    fs::create_dir_all(&plugin_dir).unwrap();

    let cue_path = write_test_disc(&input_dir);
    let _plugin_path =
        copy_named_fixture_plugin_to_tempdir("test-malformed-response-encoder.sh", &plugin_dir);

    let plugin_registry = load_plugin_registry(&plugin_dir).unwrap();
    assert!(
        !plugin_registry.is_empty(),
        "expected malformed-response fixture plugin registry to contain at least one plugin"
    );

    let source = FileInputSource::new(&cue_path);
    let components = default_pipeline_components().unwrap();
    let presentation = disc_presentation("Plugin Test.chd", Format::Chd);

    let error = prepare_mount_session_from_presentation(
        &source,
        components.identifier.as_ref(),
        components.decoder.as_ref(),
        &presentation,
        &components.policy,
        Some(&plugin_registry),
    )
    .unwrap_err();

    let message = error.to_string();

    assert!(
        message.contains("Plugin Test.chd") || message.contains("plugin-test-disc"),
        "expected error to mention requested artifact, got: {message}"
    );
    assert!(
        message.contains("plugin.fixture.malformed")
            || message.contains("fixture.disc.malformed")
            || message.contains("test-malformed-response-encoder.sh"),
        "expected error to mention selected plugin context, got: {message}"
    );
    assert!(
        message.contains("invalid")
            || message.contains("json")
            || message.contains("unexpected")
            || message.contains("response"),
        "expected error to describe malformed runtime response, got: {message}"
    );
    assert!(
        !message.contains("Failed to open config file"),
        "expected mount preparation failure to preserve runtime plugin context, got: {message}"
    );
}
