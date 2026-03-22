use std::collections::{HashMap, HashSet};

use crate::core::content::Content;
use crate::core::vfs::{VfsDirectory, VfsFile, VfsNode};
use crate::output::encode::{EncodedFile, OutputEncoder};
use crate::output::present::OutputPresenter;

pub struct GenericPresenter<E>
where
    E: OutputEncoder,
{
    encoder: E,
}

#[derive(Debug, Clone)]
struct PresentedEntry {
    content: Content,
    encoded: EncodedFile,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct DiscGroupKey {
    parent: String,
    title: String,
}

impl<E> GenericPresenter<E>
where
    E: OutputEncoder,
{
    pub fn new(encoder: E) -> Self {
        Self { encoder }
    }

    fn encode_entries(&self, content: &[Content]) -> Vec<PresentedEntry> {
        content
            .iter()
            .filter(|item| self.encoder.can_encode(item))
            .filter_map(|item| {
                self.encoder
                    .encode(item)
                    .ok()
                    .map(|encoded| PresentedEntry {
                        content: item.clone(),
                        encoded,
                    })
            })
            .collect()
    }

    fn disc_group_key(content: &crate::core::content::DiscContent) -> DiscGroupKey {
        DiscGroupKey {
            parent: Self::logical_parent_path(content.id.0.as_ref()),
            title: content.title.clone(),
        }
    }

    fn logical_parent_path(path: &str) -> String {
        let normalized = path.replace('\\', "/");

        match normalized.rsplit_once('/') {
            Some((parent, _)) => parent.to_string(),
            None => String::new(),
        }
    }

    fn build_root_children(&self, entries: &[PresentedEntry]) -> Vec<VfsNode> {
        let multi_disc_groups = self.multi_disc_groups(entries);
        let mut emitted_groups = HashSet::new();
        let mut children = Vec::new();

        for entry in entries {
            match &entry.content {
                Content::Disc(disc) => {
                    let key = Self::disc_group_key(disc);

                    if multi_disc_groups.contains(&key) {
                        if emitted_groups.insert(key.clone()) {
                            children.push(VfsNode::Directory(
                                self.build_multi_disc_directory(&key, entries),
                            ));
                        }
                    } else {
                        children.push(VfsNode::File(VfsFile::source_backed(
                            entry.encoded.name.clone(),
                            entry.encoded.size,
                            entry.content.source().clone(),
                        )));
                    }
                }
                _ => children.push(VfsNode::File(VfsFile::source_backed(
                    entry.encoded.name.clone(),
                    entry.encoded.size,
                    entry.content.source().clone(),
                ))),
            }
        }

        children
    }

    fn multi_disc_groups(&self, entries: &[PresentedEntry]) -> HashSet<DiscGroupKey> {
        let mut disc_counts: HashMap<DiscGroupKey, usize> = HashMap::new();

        for entry in entries {
            if let Content::Disc(disc) = &entry.content {
                let key = Self::disc_group_key(disc);
                *disc_counts.entry(key).or_insert(0usize) += 1;
            }
        }

        disc_counts
            .into_iter()
            .filter_map(|(key, count)| (count > 1).then_some(key))
            .collect()
    }

    fn build_multi_disc_directory(
        &self,
        key: &DiscGroupKey,
        entries: &[PresentedEntry],
    ) -> VfsDirectory {
        let mut disc_entries: Vec<_> = entries
            .iter()
            .filter_map(|entry| match &entry.content {
                Content::Disc(disc) if Self::disc_group_key(disc) == *key => {
                    Some((disc.disc_number, entry))
                }
                _ => None,
            })
            .collect();

        disc_entries.sort_by(|(left_disc, left_entry), (right_disc, right_entry)| {
            left_disc
                .cmp(right_disc)
                .then_with(|| left_entry.encoded.name.cmp(&right_entry.encoded.name))
        });

        let disc_file_names: Vec<String> = disc_entries
            .iter()
            .map(|(_, entry)| entry.encoded.name.clone())
            .collect();

        let mut children: Vec<VfsNode> = disc_entries
            .into_iter()
            .map(|(_, entry)| {
                VfsNode::File(VfsFile::source_backed(
                    entry.encoded.name.clone(),
                    entry.encoded.size,
                    entry.content.source().clone(),
                ))
            })
            .collect();

        children.push(VfsNode::File(
            self.build_m3u_file(&key.title, &disc_file_names),
        ));

        VfsDirectory::with_children(&key.title, children)
    }

    fn build_m3u_file(&self, title: &str, disc_file_names: &[String]) -> VfsFile {
        let playlist = disc_file_names.join("\n") + "\n";
        VfsFile::inline(format!("{title}.m3u"), playlist.into_bytes())
    }
}

impl<E> OutputPresenter for GenericPresenter<E>
where
    E: OutputEncoder + Send + Sync,
{
    fn present(&self, content: &[Content]) -> VfsDirectory {
        let entries = self.encode_entries(content);
        let children = self.build_root_children(&entries);

        VfsDirectory::with_children("", children)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::content::{
        BytesContent, Content, ContentId, DiscContent, RomContent, TextContent,
    };
    use crate::core::source::SourceRef;
    use crate::core::vfs::FileBacking;
    use crate::output::basic_encoder::BasicEncoder;

    #[test]
    fn presents_mixed_content_as_root_files() {
        let presenter = GenericPresenter::new(BasicEncoder::new());

        let content = vec![
            Content::Bytes(BytesContent {
                id: ContentId::new("bios"),
                source: SourceRef::new("file:/roms/bios"),
                size: 512,
            }),
            Content::Rom(RomContent {
                id: ContentId::new("sonic"),
                source: SourceRef::new("zip:/roms/megadrive.zip#sonic.bin"),
                file_name: "Sonic the Hedgehog.bin".to_string(),
                size: 1024,
            }),
            Content::Disc(DiscContent {
                id: ContentId::new("ff7-disc1"),
                source: SourceRef::new("cue:/roms/ff7.cue"),
                title: "Final Fantasy VII".to_string(),
                disc_number: 1,
                consumed_sources: vec![SourceRef::new("cue:/roms/ff7.bin")],
            }),
            Content::Text(TextContent {
                id: ContentId::new("manifest"),
                source: SourceRef::new("file:/roms/manifest"),
                size: 64,
            }),
        ];

        let root = presenter.present(&content);

        assert_eq!(root.name, "");
        assert_eq!(root.children.len(), 4);

        let names: Vec<&str> = root.children.iter().map(|node| node.name()).collect();
        assert_eq!(
            names,
            vec![
                "bios.bin",
                "Sonic the Hedgehog.bin",
                "Final Fantasy VII (Disc 1).cue",
                "manifest.txt",
            ]
        );

        match &root.children[0] {
            VfsNode::File(file) => match &file.backing {
                FileBacking::Source(source) => {
                    assert_eq!(source.to_string(), "file:/roms/bios");
                }
                other => panic!("expected source backing, got {other:?}"),
            },
            other => panic!("expected file, got {other:?}"),
        }
    }

    #[test]
    fn groups_multi_disc_sets_and_generates_playlist() {
        let presenter = GenericPresenter::new(BasicEncoder::new());

        let content = vec![
            Content::Rom(RomContent {
                id: ContentId::new("crash-bandicoot"),
                source: SourceRef::new("file:/roms/Crash Bandicoot.bin"),
                file_name: "Crash Bandicoot.bin".to_string(),
                size: 1024,
            }),
            Content::Disc(DiscContent {
                id: ContentId::new("ff7/ff7-disc2"),
                source: SourceRef::new("cue:/roms/ff7-disc2.cue"),
                title: "Final Fantasy VII".to_string(),
                disc_number: 2,
                consumed_sources: vec![SourceRef::new("cue:/roms/ff7-disc2.bin")],
            }),
            Content::Disc(DiscContent {
                id: ContentId::new("ff7/ff7-disc1"),
                source: SourceRef::new("cue:/roms/ff7-disc1.cue"),
                title: "Final Fantasy VII".to_string(),
                disc_number: 1,
                consumed_sources: vec![SourceRef::new("cue:/roms/ff7-disc1.bin")],
            }),
        ];

        let root = presenter.present(&content);

        assert_eq!(root.children.len(), 2);
        assert_eq!(root.children[0].name(), "Crash Bandicoot.bin");
        assert_eq!(root.children[1].name(), "Final Fantasy VII");

        let directory = match &root.children[1] {
            VfsNode::Directory(directory) => directory,
            other => panic!("expected directory, got {other:?}"),
        };

        let child_names: Vec<&str> = directory.children.iter().map(|node| node.name()).collect();
        assert_eq!(
            child_names,
            vec![
                "Final Fantasy VII (Disc 1).cue",
                "Final Fantasy VII (Disc 2).cue",
                "Final Fantasy VII.m3u",
            ]
        );

        let playlist = match &directory.children[2] {
            VfsNode::File(file) => file,
            other => panic!("expected file, got {other:?}"),
        };

        match &playlist.backing {
            FileBacking::Inline(contents) => {
                assert_eq!(
                    contents,
                    b"Final Fantasy VII (Disc 1).cue\nFinal Fantasy VII (Disc 2).cue\n"
                );
            }
            other => panic!("expected inline backing, got {other:?}"),
        }
    }

    #[test]
    fn does_not_generate_playlist_for_single_disc_title() {
        let presenter = GenericPresenter::new(BasicEncoder::new());

        let content = vec![Content::Disc(DiscContent {
            id: ContentId::new("mgs/mgs-disc1"),
            source: SourceRef::new("cue:/roms/mgs-disc1.cue"),
            title: "Metal Gear Solid".to_string(),
            disc_number: 1,
            consumed_sources: vec![SourceRef::new("cue:/roms/mgs-disc1.bin")],
        })];

        let root = presenter.present(&content);

        assert_eq!(root.children.len(), 1);
        assert_eq!(root.children[0].name(), "Metal Gear Solid (Disc 1).cue");
    }

    #[test]
    fn does_not_merge_same_title_from_different_directories() {
        let presenter = GenericPresenter::new(BasicEncoder::new());

        let content = vec![
            Content::Disc(DiscContent {
                id: ContentId::new("discs/ps1_multi/game_disc1.cue"),
                source: SourceRef::new("file:/roms/discs/ps1_multi/game_disc1.cue"),
                title: "game".to_string(),
                disc_number: 1,
                consumed_sources: vec![SourceRef::new("file:/roms/discs/ps1_multi/game_disc1.bin")],
            }),
            Content::Disc(DiscContent {
                id: ContentId::new("discs/ps1_multi/game_disc2.cue"),
                source: SourceRef::new("file:/roms/discs/ps1_multi/game_disc2.cue"),
                title: "game".to_string(),
                disc_number: 2,
                consumed_sources: vec![SourceRef::new("file:/roms/discs/ps1_multi/game_disc2.bin")],
            }),
            Content::Disc(DiscContent {
                id: ContentId::new("discs/ps1_single/game.cue"),
                source: SourceRef::new("file:/roms/discs/ps1_single/game.cue"),
                title: "game".to_string(),
                disc_number: 1,
                consumed_sources: vec![SourceRef::new("file:/roms/discs/ps1_single/game.bin")],
            }),
        ];

        let root = presenter.present(&content);

        assert_eq!(root.children.len(), 2);

        let names: Vec<&str> = root.children.iter().map(|node| node.name()).collect();
        assert_eq!(names, vec!["game", "game (Disc 1).cue"]);

        let multi_dir = match &root.children[0] {
            VfsNode::Directory(directory) => directory,
            other => panic!("expected directory, got {other:?}"),
        };

        let child_names: Vec<&str> = multi_dir.children.iter().map(|node| node.name()).collect();
        assert_eq!(
            child_names,
            vec!["game (Disc 1).cue", "game (Disc 2).cue", "game.m3u"]
        );
    }

    #[test]
    fn normalizes_disc_group_parent_to_logical_path() {
        let key = GenericPresenter::<BasicEncoder>::disc_group_key(&DiscContent {
            id: ContentId::new(r"discs\ps1_multi\game_disc1.cue"),
            source: SourceRef::new("file:/roms/discs/ps1_multi/game_disc1.cue"),
            title: "game".to_string(),
            disc_number: 1,
            consumed_sources: vec![SourceRef::new("file:/roms/discs/ps1_multi/game_disc1.bin")],
        });
        assert_eq!(key.parent, "discs/ps1_multi");
    }
}
