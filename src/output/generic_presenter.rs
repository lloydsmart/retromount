use std::collections::{BTreeMap, HashSet};

use crate::core::content::Content;
use crate::core::vfs::{VfsDirectory, VfsFile, VfsNode};
use crate::output::encode::OutputEncoder;
use crate::output::present::OutputPresenter;

pub struct GenericPresenter<E>
where
    E: OutputEncoder,
{
    encoder: E,
}

impl<E> GenericPresenter<E>
where
    E: OutputEncoder,
{
    pub fn new(encoder: E) -> Self {
        Self { encoder }
    }

    fn present_file(&self, content: &Content) -> Option<VfsNode> {
        if !self.encoder.can_encode(content) {
            return None;
        }

        self.encoder
            .encode(content)
            .ok()
            .map(|encoded| VfsNode::File(VfsFile::new(encoded.name, encoded.size)))
    }
}

impl<E> OutputPresenter for GenericPresenter<E>
where
    E: OutputEncoder + Send + Sync,
{
    fn present(&self, content: &[Content]) -> VfsDirectory {
        let mut multi_disc_titles = HashSet::new();
        let mut discs_by_title: BTreeMap<String, Vec<&Content>> = BTreeMap::new();

        for item in content {
            if let Content::Disc(disc) = item {
                discs_by_title
                    .entry(disc.title.clone())
                    .or_default()
                    .push(item);
            }
        }

        for (title, discs) in &discs_by_title {
            if discs.len() > 1 {
                multi_disc_titles.insert(title.clone());
            }
        }

        let mut emitted_groups = HashSet::new();
        let mut children = Vec::new();

        for item in content {
            match item {
                Content::Disc(disc) if multi_disc_titles.contains(&disc.title) => {
                    if !emitted_groups.insert(disc.title.clone()) {
                        continue;
                    }

                    let mut grouped_discs =
                        discs_by_title.get(&disc.title).cloned().unwrap_or_default();

                    grouped_discs.sort_by_key(|candidate| match candidate {
                        Content::Disc(grouped_disc) => grouped_disc.disc_number,
                        _ => 0,
                    });

                    let grouped_children = grouped_discs
                        .into_iter()
                        .filter_map(|candidate| self.present_file(candidate))
                        .collect();

                    children.push(VfsNode::Directory(VfsDirectory::with_children(
                        &disc.title,
                        grouped_children,
                    )));
                }
                _ => {
                    if let Some(node) = self.present_file(item) {
                        children.push(node);
                    }
                }
            }
        }

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
    fn groups_multi_disc_content_into_directory() {
        let presenter = GenericPresenter::new(BasicEncoder::new());

        let content = vec![
            Content::Disc(DiscContent {
                id: ContentId::new("game-disc2"),
                source: SourceRef::new("cue:/roms/game_disc2.cue"),
                title: "game".to_string(),
                disc_number: 2,
                consumed_sources: vec![SourceRef::new("cue:/roms/game_disc2.bin")],
            }),
            Content::Disc(DiscContent {
                id: ContentId::new("game-disc1"),
                source: SourceRef::new("cue:/roms/game_disc1.cue"),
                title: "game".to_string(),
                disc_number: 1,
                consumed_sources: vec![SourceRef::new("cue:/roms/game_disc1.bin")],
            }),
        ];

        let root = presenter.present(&content);

        assert_eq!(root.children.len(), 1);

        match &root.children[0] {
            VfsNode::Directory(dir) => {
                assert_eq!(dir.name, "game");
                assert_eq!(dir.children.len(), 2);

                let names: Vec<&str> = dir.children.iter().map(|node| node.name()).collect();
                assert_eq!(names, vec!["game (Disc 1).cue", "game (Disc 2).cue"]);
            }
            other => panic!("expected grouped directory, got {other:?}"),
        }
    }

    #[test]
    fn preserves_mixed_root_content_around_grouped_multi_disc_set() {
        let presenter = GenericPresenter::new(BasicEncoder::new());

        let content = vec![
            Content::Rom(RomContent {
                id: ContentId::new("sonic"),
                source: SourceRef::new("zip:/roms/megadrive.zip#sonic.bin"),
                file_name: "Sonic the Hedgehog.bin".to_string(),
                size: 1024,
            }),
            Content::Disc(DiscContent {
                id: ContentId::new("game-disc1"),
                source: SourceRef::new("cue:/roms/game_disc1.cue"),
                title: "game".to_string(),
                disc_number: 1,
                consumed_sources: vec![SourceRef::new("cue:/roms/game_disc1.bin")],
            }),
            Content::Disc(DiscContent {
                id: ContentId::new("game-disc2"),
                source: SourceRef::new("cue:/roms/game_disc2.cue"),
                title: "game".to_string(),
                disc_number: 2,
                consumed_sources: vec![SourceRef::new("cue:/roms/game_disc2.bin")],
            }),
            Content::Text(TextContent {
                id: ContentId::new("manifest"),
                source: SourceRef::new("file:/roms/manifest"),
                size: 64,
            }),
        ];

        let root = presenter.present(&content);

        let names: Vec<&str> = root.children.iter().map(|node| node.name()).collect();
        assert_eq!(
            names,
            vec!["Sonic the Hedgehog.bin", "game", "manifest.txt"]
        );

        match &root.children[1] {
            VfsNode::Directory(dir) => {
                let child_names: Vec<&str> = dir.children.iter().map(|node| node.name()).collect();
                assert_eq!(child_names, vec!["game (Disc 1).cue", "game (Disc 2).cue"]);
            }
            other => panic!("expected grouped directory, got {other:?}"),
        }
    }
}
