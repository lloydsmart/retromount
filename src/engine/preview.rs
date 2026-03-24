use std::io::{self, Write};
use std::path::Path;

use crate::core::vfs::{VfsDirectory, VfsNode};
use crate::error::RetromountError;
use crate::input::basic_decoder::BasicInputDecoder;
use crate::input::basic_identifier::BasicInputIdentifier;
use crate::input::directory_source::DirectoryInputSource;
use crate::input::source::InputSource;
use crate::input::zip_source::ZipInputSource;
use crate::output::basic_encoder::BasicEncoder;
use crate::output::generic_presenter::GenericPresenter;

use super::pipeline::run_pipeline;

pub fn run_phase3_preview(path: &Path) -> Result<(), RetromountError> {
    let source = build_input_source(path)?;

    let identifier = BasicInputIdentifier::new();
    let decoder = BasicInputDecoder::new();
    let presenter = GenericPresenter::new(BasicEncoder::new());

    let root = run_pipeline(source.as_ref(), &identifier, &decoder, &presenter)?;

    let stdout = io::stdout();
    let mut handle = stdout.lock();
    write_vfs_tree(&mut handle, &root)?;

    Ok(())
}

pub fn build_input_source(path: &Path) -> Result<Box<dyn InputSource>, RetromountError> {
    if path.is_dir() {
        return Ok(Box::new(DirectoryInputSource::new(path)));
    }

    if path.is_file()
        && path
            .extension()
            .and_then(|ext| ext.to_str())
            .is_some_and(|ext| ext.eq_ignore_ascii_case("zip"))
    {
        return Ok(Box::new(ZipInputSource::new(path)));
    }

    Err(RetromountError::UnsupportedFormat)
}

pub fn write_vfs_tree<W: Write>(writer: &mut W, root: &VfsDirectory) -> io::Result<()> {
    writeln!(writer, "/")?;

    for child in root.children() {
        write_vfs_node(writer, child, 1)?;
    }

    Ok(())
}

fn write_vfs_node<W: Write>(writer: &mut W, node: &VfsNode, depth: usize) -> io::Result<()> {
    let indent = "  ".repeat(depth);

    match node {
        VfsNode::File(file) => {
            writeln!(writer, "{indent}{}", file.name)?;
        }
        VfsNode::Directory(dir) => {
            writeln!(writer, "{indent}{}/", dir.name)?;

            for child in dir.children() {
                write_vfs_node(writer, child, depth + 1)?;
            }
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::vfs::{VfsDirectory, VfsFile, VfsNode};

    #[test]
    fn writes_simple_vfs_tree() {
        let root = VfsDirectory::with_children(
            "",
            vec![
                VfsNode::File(VfsFile::new("mario.sfc")),
                VfsNode::File(VfsFile::new("readme.txt")),
                VfsNode::File(VfsFile::new("blob.dat.bin")),
            ],
        );

        let mut output = Vec::new();
        write_vfs_tree(&mut output, &root).unwrap();

        let rendered = String::from_utf8(output).unwrap();
        assert_eq!(rendered, "/\n  blob.dat.bin\n  mario.sfc\n  readme.txt\n");
    }

    #[test]
    fn writes_nested_vfs_tree() {
        let root = VfsDirectory::with_children(
            "",
            vec![VfsNode::Directory(VfsDirectory::with_children(
                "snes",
                vec![VfsNode::File(VfsFile::new("zelda.sfc"))],
            ))],
        );

        let mut output = Vec::new();
        write_vfs_tree(&mut output, &root).unwrap();

        let rendered = String::from_utf8(output).unwrap();
        assert_eq!(rendered, "/\n  snes/\n    zelda.sfc\n");
    }
}
