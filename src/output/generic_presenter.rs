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

    fn build_root_children(&self, entries: &[PresentedEntry]) -> Vec<VfsNode> {
        let multi_disc_titles = self.multi_disc_titles(entries);
        let mut emitted_groups = HashSet::new();
        let mut children = Vec::new();

        for entry in entries {
            match &entry.content {
                Content::Disc(disc) if multi_disc_titles.contains(&disc.title) => {
                    if emitted_groups.insert(disc.title.clone()) {
                        children.push(VfsNode::Directory(
                            self.build_multi_disc_directory(&disc.title, entries),
                        ));
                    }
                }
                _ => children.push(VfsNode::File(VfsFile::new(
                    entry.encoded.name.clone(),
                    entry.encoded.size,
                ))),
            }
        }

        children
    }

    fn multi_disc_titles(&self, entries: &[PresentedEntry]) -> HashSet<String> {
        let mut disc_counts = HashMap::new();

        for entry in entries {
            if let Content::Disc(disc) = &entry.content {
                *disc_counts.entry(disc.title.clone()).or_insert(0usize) += 1;
            }
        }

        disc_counts
            .into_iter()
            .filter_map(|(title, count)| (count > 1).then_some(title))
            .collect()
    }

    fn build_multi_disc_directory(&self, title: &str, entries: &[PresentedEntry]) -> VfsDirectory {
        let mut disc_entries: Vec<_> = entries
            .iter()
            .filter_map(|entry| match &entry.content {
                Content::Disc(disc) if disc.title == title => Some((disc.disc_number, entry)),
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
                VfsNode::File(VfsFile::new(entry.encoded.name.clone(), entry.encoded.size))
            })
            .collect();

        children.push(VfsNode::File(self.build_m3u_file(title, &disc_file_names)));

        VfsDirectory::with_children(title, children)
    }

    fn build_m3u_file(&self, title: &str, disc_file_names: &[String]) -> VfsFile {
        let playlist = disc_file_names.join("\n") + "\n";
        VfsFile::with_contents(format!("{title}.m3u"), playlist.into_bytes())
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
                id: ContentId::new("ff7-disc2"),
                source: SourceRef::new("cue:/roms/ff7-disc2.cue"),
                title: "Final Fantasy VII".to_string(),
                disc_number: 2,
                consumed_sources: vec![SourceRef::new("cue:/roms/ff7-disc2.bin")],
            }),
            Content::Disc(DiscContent {
                id: ContentId::new("ff7-disc1"),
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

        assert_eq!(
            playlist.contents,
            Some(b"Final Fantasy VII (Disc 1).cue\nFinal Fantasy VII (Disc 2).cue\n".to_vec())
        );
    }

    #[test]
    fn does_not_generate_playlist_for_single_disc_title() {
        let presenter = GenericPresenter::new(BasicEncoder::new());

        let content = vec![Content::Disc(DiscContent {
            id: ContentId::new("metal-gear-solid"),
            source: SourceRef::new("cue:/roms/mgs-disc1.cue"),
            title: "Metal Gear Solid".to_string(),
            disc_number: 1,
            consumed_sources: vec![SourceRef::new("cue:/roms/mgs-disc1.bin")],
        })];

        let root = presenter.present(&content);

        assert_eq!(root.children.len(), 1);
        assert_eq!(root.children[0].name(), "Metal Gear Solid (Disc 1).cue");
    }
}
