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
}

impl<E> OutputPresenter for GenericPresenter<E>
where
    E: OutputEncoder + Send + Sync,
{
    fn present(&self, content: &[Content]) -> VfsDirectory {
        let children = content
            .iter()
            .filter(|item| self.encoder.can_encode(item))
            .filter_map(|item| {
                self.encoder
                    .encode(item)
                    .ok()
                    .map(|encoded| VfsNode::File(VfsFile::new(encoded.name, encoded.size)))
            })
            .collect();

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
            })
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
                "Metal Gear Solid (Disc 1).cue",
                "manifest.txt",
            ]
        );
    }
}
