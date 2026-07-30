use std::io;
use std::path::Path;

use crate::core::input_content::{InputAccess, InputContent};
use crate::core::reader::Reader;
use crate::core::reader_handle::ReaderHandle;
use crate::core::source::SourceRef;
use crate::readers::dir_reader::DirReader;
use crate::readers::zip_reader::ZipReader;

pub fn filesystem_content(path: &Path) -> io::Result<InputContent> {
    let size = std::fs::metadata(path)?.len();
    let reader_path = path.to_path_buf();

    Ok(InputContent::new(
        size,
        InputAccess::RandomAccess,
        ReaderHandle::new(format!("file:{}", path.to_string_lossy()), move || {
            Ok(Box::new(DirReader::open(&reader_path)?))
        }),
    ))
}

pub fn zip_entry_content(
    archive_path: &Path,
    entry_name: &str,
    size: u64,
    access: InputAccess,
) -> InputContent {
    let reader_archive_path = archive_path.to_path_buf();
    let reader_entry_name = entry_name.to_string();

    InputContent::new(
        size,
        access,
        ReaderHandle::new(
            format!(
                "zip:{}#{}",
                reader_archive_path.to_string_lossy(),
                reader_entry_name
            ),
            move || {
                Ok(Box::new(ZipReader::open(
                    &reader_archive_path,
                    &reader_entry_name,
                )?))
            },
        ),
    )
}

/// Resolves stable downstream source identities through the same central
/// filesystem/container boundary used during input enumeration.
pub fn open_source_ref(source: &SourceRef) -> io::Result<Box<dyn Reader>> {
    let source = source.0.as_ref();

    if let Some(remainder) = source.strip_prefix("zip:") {
        let (archive_path, entry_name) = remainder.split_once('#').ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::InvalidInput,
                format!("invalid ZIP source ref: {source}"),
            )
        })?;

        return Ok(Box::new(ZipReader::open(
            Path::new(archive_path),
            entry_name,
        )?));
    }

    let path = if let Some(path) = source.strip_prefix("file:") {
        path
    } else if let Some(path) = source.strip_prefix("cue:") {
        path
    } else {
        source
    };

    Ok(Box::new(DirReader::open(Path::new(path))?))
}

pub fn resolve_source_ref(source: &SourceRef) -> io::Result<InputContent> {
    let raw = source.0.as_ref();

    if let Some(remainder) = raw.strip_prefix("zip:") {
        let (archive_path, entry_name) = remainder.split_once('#').ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::InvalidInput,
                format!("invalid ZIP source ref: {raw}"),
            )
        })?;
        let file = std::fs::File::open(archive_path)?;
        let mut archive = zip::ZipArchive::new(file)
            .map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error))?;
        let entry = archive
            .by_name(entry_name)
            .map_err(|error| io::Error::new(io::ErrorKind::NotFound, error))?;
        let access = if entry.compression() == zip::CompressionMethod::Stored {
            InputAccess::RandomAccess
        } else {
            InputAccess::Sequential
        };

        return Ok(zip_entry_content(
            Path::new(archive_path),
            entry_name,
            entry.size(),
            access,
        ));
    }

    let path = raw
        .strip_prefix("file:")
        .or_else(|| raw.strip_prefix("cue:"))
        .unwrap_or(raw);
    filesystem_content(Path::new(path))
}
