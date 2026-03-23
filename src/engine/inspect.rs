use std::io::{self, Write};
use std::path::Path;

use crate::core::content::{Content, GamePart};
use crate::error::RetromountError;
use crate::input::basic_decoder::BasicInputDecoder;
use crate::input::basic_identifier::BasicInputIdentifier;
use crate::output::basic_encoder::BasicEncoder;
use crate::output::generic_presenter::GenericPresenter;

use super::pipeline::{run_pipeline_with_trace, PipelineTrace};
use super::preview::{build_input_source, write_vfs_tree};

pub fn run_phase3_inspect(path: &Path, json: bool) -> Result<(), RetromountError> {
    let source = build_input_source(path)?;
    let kind = source.kind();

    let identifier = BasicInputIdentifier::new();
    let decoder = BasicInputDecoder::new();
    let presenter = GenericPresenter::new(BasicEncoder::new());

    let trace = run_pipeline_with_trace(source.as_ref(), &identifier, &decoder, &presenter)?;

    if json {
        let output = serde_json::to_string_pretty(&trace)
            .map_err(|err| RetromountError::LoadError(err.to_string()))?;
        println!("{output}");
        return Ok(());
    }

    let stdout = io::stdout();
    let mut handle = stdout.lock();

    write_inspect_report(&mut handle, path, kind.as_str(), &trace)?;

    Ok(())
}

pub fn write_inspect_report<W: Write>(
    writer: &mut W,
    path: &Path,
    input_kind: &str,
    trace: &PipelineTrace,
) -> io::Result<()> {
    writeln!(writer, "Input:")?;
    writeln!(writer, "  Path: {}", path.display())?;
    writeln!(writer, "  Type: {}", input_kind)?;
    writeln!(writer, "  Objects: {}", trace.objects.len())?;
    writeln!(writer)?;

    writeln!(writer, "Decoded:")?;
    if trace.objects.is_empty() {
        writeln!(writer, "  (none)")?;
    } else {
        for object in &trace.objects {
            writeln!(writer, "  - {}", object.object.name)?;
            writeln!(writer, "    Source: {}", object.object.source)?;
            writeln!(writer, "    Identity: {:?}", object.identity)?;
            writeln!(writer, "    Supported: {}", yes_no(object.supported))?;

            if object.decoded.is_empty() {
                writeln!(writer, "    Decoded: (none)")?;
            } else {
                for content in &object.decoded {
                    write_content_summary(writer, content)?;
                }
            }
        }
    }

    writeln!(writer)?;
    writeln!(writer, "Normalized:")?;
    if trace.normalized.is_empty() {
        writeln!(writer, "  (none)")?;
    } else {
        for content in &trace.normalized {
            writeln!(writer, "  - {:?}", content.kind())?;
            write_content_summary(writer, content)?;
        }
    }

    writeln!(writer)?;
    writeln!(writer, "Presented VFS:")?;
    write_vfs_tree(writer, &trace.presented)?;

    Ok(())
}

fn write_content_summary<W: Write>(writer: &mut W, content: &Content) -> io::Result<()> {
    match content {
        Content::Bytes(bytes) => {
            writeln!(writer, "    Decoded: Bytes")?;
            writeln!(writer, "      ID: {}", bytes.id)?;
            writeln!(writer, "      Source: {}", bytes.source)?;
            writeln!(writer, "      Size: {}", bytes.size)?;
        }
        Content::Rom(rom) => {
            writeln!(writer, "    Decoded: Rom")?;
            writeln!(writer, "      ID: {}", rom.id)?;
            writeln!(writer, "      Source: {}", rom.source)?;
            writeln!(writer, "      File name: {}", rom.file_name)?;
            writeln!(writer, "      Size: {}", rom.size)?;
        }
        Content::Disc(disc) => {
            writeln!(writer, "    Decoded: Disc")?;
            writeln!(writer, "      ID: {}", disc.id)?;
            writeln!(writer, "      Source: {}", disc.source)?;
            writeln!(writer, "      Title: {}", disc.title)?;
            writeln!(writer, "      Disc number: {}", disc.disc_number)?;
        }
        Content::Game(game) => {
            writeln!(writer, "    Decoded: Game")?;
            writeln!(writer, "      ID: {}", game.id)?;
            writeln!(writer, "      Source: {}", game.source)?;
            writeln!(writer, "      Title: {}", game.title)?;
            writeln!(writer, "      Platform: {}", game.platform)?;
            writeln!(writer, "      Parts: {}", game.parts.len())?;

            for (index, part) in game.parts.iter().enumerate() {
                match part {
                    GamePart::Rom(rom) => {
                        writeln!(writer, "      Part {}: Rom", index + 1)?;
                        writeln!(writer, "        Source: {}", rom.source)?;
                        writeln!(writer, "        File name: {}", rom.file_name)?;
                        writeln!(writer, "        Size: {}", rom.size)?;
                    }
                    GamePart::Disc(disc) => {
                        writeln!(writer, "      Part {}: Disc", index + 1)?;
                        writeln!(writer, "        Source: {}", disc.source)?;
                        writeln!(writer, "        Disc number: {}", disc.disc_number)?;
                    }
                }
            }
        }
        Content::Text(text) => {
            writeln!(writer, "    Decoded: Text")?;
            writeln!(writer, "      ID: {}", text.id)?;
            writeln!(writer, "      Source: {}", text.source)?;
            writeln!(writer, "      Size: {}", text.size)?;
        }
    }

    Ok(())
}

fn yes_no(value: bool) -> &'static str {
    if value {
        "yes"
    } else {
        "no"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::content::{BytesContent, ContentId};
    use crate::core::source::{SourceObject, SourceRef};
    use crate::core::vfs::{VfsDirectory, VfsFile, VfsNode};
    use crate::engine::pipeline::TracedObject;
    use crate::input::identify::InputIdentity;

    #[test]
    fn writes_inspect_report() {
        let trace = PipelineTrace {
            objects: vec![TracedObject {
                object: SourceObject {
                    source: SourceRef::new("zip:/tmp/library.zip#misc/blob.dat"),
                    name: "blob.dat".to_string(),
                },
                identity: InputIdentity::File,
                supported: true,
                decoded: vec![Content::Bytes(BytesContent {
                    id: ContentId::new("blob.dat"),
                    source: SourceRef::new("zip:/tmp/library.zip#misc/blob.dat"),
                    size: 0,
                })],
            }],
            normalized: vec![Content::Bytes(BytesContent {
                id: ContentId::new("blob.dat"),
                source: SourceRef::new("zip:/tmp/library.zip#misc/blob.dat"),
                size: 0,
            })],
            presented: VfsDirectory::with_children(
                "",
                vec![VfsNode::File(VfsFile::new("blob.dat.bin"))],
            ),
        };

        let mut output = Vec::new();
        write_inspect_report(&mut output, Path::new("/tmp/library.zip"), "Zip", &trace).unwrap();

        let rendered = String::from_utf8(output).unwrap();

        assert!(rendered.contains("Input:"));
        assert!(rendered.contains("Path: /tmp/library.zip"));
        assert!(rendered.contains("Type: Zip"));
        assert!(rendered.contains("Objects: 1"));
        assert!(rendered.contains("Decoded:"));
        assert!(rendered.contains("Identity: File"));
        assert!(rendered.contains("Supported: yes"));
        assert!(rendered.contains("Decoded: Bytes"));
        assert!(rendered.contains("Normalized:"));
        assert!(rendered.contains("Presented VFS:"));
        assert!(rendered.contains("blob.dat.bin"));
    }
}
