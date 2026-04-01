use std::io::{self, Write};
use std::path::Path;

use crate::core::content::{DecodedContent, GamePart, NormalizedContent};
use crate::engine::components::pipeline_components_for_presenter;
use crate::error::RetromountError;
use crate::output::present::PresenterKind;

use super::pipeline::{run_pipeline_with_trace, PipelineTrace};
use super::preview::{build_input_source, write_vfs_tree};

pub fn run_phase3_inspect(
    path: &Path,
    json: bool,
    presenter_kind: PresenterKind,
) -> Result<(), RetromountError> {
    let source = build_input_source(path)?;
    let kind = source.kind();
    let components = pipeline_components_for_presenter(presenter_kind);
    let trace = run_pipeline_with_trace(
        source.as_ref(),
        components.identifier.as_ref(),
        components.decoder.as_ref(),
        components.presenter.as_ref(),
    )?;

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
                    write_decoded_content_summary(writer, content)?;
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
            write_normalized_content_summary(writer, content)?;
        }
    }

    writeln!(writer)?;
    writeln!(writer, "Output VFS:")?;
    write_vfs_tree(writer, &trace.output_vfs)?;

    Ok(())
}

fn write_normalized_content_summary<W: Write>(
    writer: &mut W,
    content: &NormalizedContent,
) -> io::Result<()> {
    match content {
        NormalizedContent::Bytes(bytes) => {
            writeln!(writer, "    Content: Bytes")?;
            writeln!(writer, "      ID: {}", bytes.id)?;
            writeln!(writer, "      Source: {}", bytes.source)?;
            writeln!(writer, "      Size: {}", bytes.size)?;
        }
        NormalizedContent::Game(game) => {
            writeln!(writer, "    Content: Game")?;
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
                        writeln!(writer, "        File name: {}", rom.source.file_name())?;
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
        NormalizedContent::Text(text) => {
            writeln!(writer, "    Content: Text")?;
            writeln!(writer, "      ID: {}", text.id)?;
            writeln!(writer, "      Source: {}", text.source)?;
            writeln!(writer, "      Size: {}", text.size)?;
        }
    }

    Ok(())
}

fn write_decoded_content_summary<W: Write>(
    writer: &mut W,
    content: &DecodedContent,
) -> io::Result<()> {
    match content {
        DecodedContent::Bytes(bytes) => {
            writeln!(writer, "    Decoded: Bytes")?;
            writeln!(writer, "      ID: {}", bytes.id)?;
            writeln!(writer, "      Source: {}", bytes.source)?;
            writeln!(writer, "      Size: {}", bytes.size)?;
        }
        DecodedContent::Rom(rom) => {
            writeln!(writer, "    Decoded: Rom")?;
            writeln!(writer, "      ID: {}", rom.id)?;
            writeln!(writer, "      Source: {}", rom.source)?;
            writeln!(writer, "      File name: {}", rom.source.file_name())?;
            writeln!(writer, "      Size: {}", rom.size)?;
        }
        DecodedContent::Disc(disc) => {
            writeln!(writer, "    Decoded: Disc")?;
            writeln!(writer, "      ID: {}", disc.id)?;
            writeln!(writer, "      Source: {}", disc.source)?;
            writeln!(writer, "      Title: {}", disc.title)?;
            writeln!(writer, "      Disc number: {}", disc.disc_number)?;
        }
        DecodedContent::Text(text) => {
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
