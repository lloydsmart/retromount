use std::collections::HashMap;
use std::io;

use crate::core::vfs::{VfsDirectory, VfsFile, VfsNode};
use crate::output::plan::{
    ArtifactId, GeneratedArtifact, PlanEntry, PlanFile, PlannedArtifactKind, PlaylistArtifact,
    PresentationPlan, SourceArtifact,
};

pub fn materialize_plan(plan: &PresentationPlan) -> Result<VfsDirectory, io::Error> {
    let artifact_names = collect_artifact_names(&plan.entries);
    let children = materialize_entries(&plan.entries, &artifact_names)?;
    Ok(VfsDirectory::with_children("", children))
}

fn collect_artifact_names(entries: &[PlanEntry]) -> HashMap<ArtifactId, String> {
    let mut names = HashMap::new();
    collect_artifact_names_recursive(entries, &mut names);
    names
}

fn collect_artifact_names_recursive(
    entries: &[PlanEntry],
    names: &mut HashMap<ArtifactId, String>,
) {
    for entry in entries {
        match entry {
            PlanEntry::Directory(dir) => collect_artifact_names_recursive(&dir.entries, names),
            PlanEntry::File(file) => {
                names.insert(file.artifact.id.clone(), file.name.clone());
            }
        }
    }
}

fn materialize_entries(
    entries: &[PlanEntry],
    artifact_names: &HashMap<ArtifactId, String>,
) -> Result<Vec<VfsNode>, io::Error> {
    let mut children = Vec::with_capacity(entries.len());

    for entry in entries {
        match entry {
            PlanEntry::Directory(dir) => {
                let nested_children = materialize_entries(&dir.entries, artifact_names)?;
                children.push(VfsNode::Directory(VfsDirectory::with_children(
                    &dir.name,
                    nested_children,
                )));
            }
            PlanEntry::File(file) => {
                children.push(VfsNode::File(materialize_file(file, artifact_names)?));
            }
        }
    }

    Ok(children)
}

fn materialize_file(
    file: &PlanFile,
    artifact_names: &HashMap<ArtifactId, String>,
) -> Result<VfsFile, io::Error> {
    match &file.artifact.kind {
        PlannedArtifactKind::SourceBacked(source_artifact) => {
            materialize_source_backed_file(&file.name, source_artifact)
        }
        PlannedArtifactKind::Generated(generated) => {
            materialize_generated_file(&file.name, generated, artifact_names)
        }
    }
}

fn materialize_source_backed_file(
    name: &str,
    artifact: &SourceArtifact,
) -> Result<VfsFile, io::Error> {
    match artifact.inputs.as_slice() {
        [input] => Ok(VfsFile::source_backed(
            name,
            input.size,
            input.source.clone(),
        )),
        _ => Err(io::Error::new(
            io::ErrorKind::Unsupported,
            "multi-source source-backed artifact materialization is not supported",
        )),
    }
}

fn materialize_generated_file(
    name: &str,
    artifact: &GeneratedArtifact,
    artifact_names: &HashMap<ArtifactId, String>,
) -> Result<VfsFile, io::Error> {
    match artifact {
        GeneratedArtifact::Playlist(playlist) => {
            let contents = materialize_playlist_contents(playlist, artifact_names)?;
            Ok(VfsFile::inline(name, contents))
        }
    }
}

fn materialize_playlist_contents(
    playlist: &PlaylistArtifact,
    artifact_names: &HashMap<ArtifactId, String>,
) -> Result<Vec<u8>, io::Error> {
    let mut lines = Vec::with_capacity(playlist.entries.len());

    for entry in &playlist.entries {
        let name = artifact_names.get(&entry.artifact_id).ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::InvalidData,
                format!(
                    "playlist references unknown artifact '{}'",
                    entry.artifact_id.0
                ),
            )
        })?;

        lines.push(name.clone());
    }

    let mut text = lines.join("\n");
    text.push('\n');

    Ok(text.into_bytes())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::source::SourceRef;
    use crate::output::capabilities::{CapabilityRequirements, ContentType, Format};
    use crate::output::plan::{
        ArtifactReference, ArtifactRequest, GeneratedArtifact, PlanFile, PlannedArtifactKind,
        PlaylistArtifact, SourceArtifact, SourceArtifactInput,
    };

    #[test]
    fn materializes_source_backed_file() {
        let plan = PresentationPlan::new(vec![PlanEntry::File(PlanFile::new(
            "game.bin",
            ArtifactRequest::new(
                ArtifactId::new("game"),
                PlannedArtifactKind::SourceBacked(SourceArtifact::single(
                    SourceRef::new("file:/roms/game.bin"),
                    1024,
                )),
                CapabilityRequirements::new(ContentType::Rom).with_format(Format::Bin),
            ),
        ))]);

        let root = materialize_plan(&plan).unwrap();

        assert_eq!(root.children().len(), 1);
        assert_eq!(root.children()[0].name(), "game.bin");
    }

    #[test]
    fn materializes_playlist_file_from_referenced_artifact_names() {
        let disc1_id = ArtifactId::new("disc1");
        let disc2_id = ArtifactId::new("disc2");

        let plan = PresentationPlan::new(vec![
            PlanEntry::File(PlanFile::new(
                "Game (Disc 1).cue",
                ArtifactRequest::new(
                    disc1_id.clone(),
                    PlannedArtifactKind::SourceBacked(SourceArtifact::single(
                        SourceRef::new("file:/roms/disc1.cue"),
                        100,
                    )),
                    CapabilityRequirements::new(ContentType::Disc).with_format(Format::Bin),
                ),
            )),
            PlanEntry::File(PlanFile::new(
                "Game (Disc 2).cue",
                ArtifactRequest::new(
                    disc2_id.clone(),
                    PlannedArtifactKind::SourceBacked(SourceArtifact::single(
                        SourceRef::new("file:/roms/disc2.cue"),
                        200,
                    )),
                    CapabilityRequirements::new(ContentType::Disc).with_format(Format::Bin),
                ),
            )),
            PlanEntry::File(PlanFile::new(
                "Game.m3u",
                ArtifactRequest::new(
                    ArtifactId::new("playlist"),
                    PlannedArtifactKind::Generated(GeneratedArtifact::Playlist(
                        PlaylistArtifact::new(vec![
                            ArtifactReference::new(disc1_id),
                            ArtifactReference::new(disc2_id),
                        ]),
                    )),
                    CapabilityRequirements::new(ContentType::Playlist).with_format(Format::M3u),
                ),
            )),
        ]);

        let root = materialize_plan(&plan).unwrap();

        let playlist = match &root.children()[2] {
            VfsNode::File(file) => file,
            other => panic!("expected file, got {other:?}"),
        };

        match &playlist.backing {
            crate::core::vfs::FileBacking::Inline(contents) => {
                assert_eq!(contents, b"Game (Disc 1).cue\nGame (Disc 2).cue\n");
            }
            other => panic!("expected inline backing, got {other:?}"),
        }
    }

    #[test]
    fn rejects_multi_source_source_backed_artifact() {
        let plan = PresentationPlan::new(vec![PlanEntry::File(PlanFile::new(
            "merged.bin",
            ArtifactRequest::new(
                ArtifactId::new("merged"),
                PlannedArtifactKind::SourceBacked(SourceArtifact::multiple(vec![
                    SourceArtifactInput {
                        source: SourceRef::new("file:/roms/part1.bin"),
                        size: 100,
                    },
                    SourceArtifactInput {
                        source: SourceRef::new("file:/roms/part2.bin"),
                        size: 200,
                    },
                ])),
                CapabilityRequirements::new(ContentType::Archive).with_format(Format::Bin),
            ),
        ))]);

        let error = materialize_plan(&plan).unwrap_err();
        assert_eq!(error.kind(), io::ErrorKind::Unsupported);
    }
}
