use std::io;
use std::path::{Path, PathBuf};

use crate::core::cd::CdSbi;
use crate::core::source::{SourceObject, SourceOrigin, SourceRef};
use crate::core::source_resolver::resolve_source_ref;

const SBI_HEADER: &[u8; 4] = b"SBI\0";
const SBI_RECORD_SIZE: u64 = 14;

pub fn discover_sbi_sidecar(object: &SourceObject) -> io::Result<Option<CdSbi>> {
    let candidates = match &object.origin {
        SourceOrigin::Filesystem(path) => filesystem_candidates(path)?,
        SourceOrigin::ZipEntry {
            archive_path,
            entry_name,
        } => zip_candidates(archive_path, entry_name)?,
    };

    let source = match candidates.as_slice() {
        [] => return Ok(None),
        [source] => source.clone(),
        _ => {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!("disc '{}' has multiple matching SBI sidecars", object.name),
            ));
        }
    };
    let content = resolve_source_ref(&source)?;
    validate_sbi(&source, &content)?;

    Ok(Some(CdSbi {
        source,
        size: content.size,
        content: content.handle,
    }))
}

fn filesystem_candidates(path: &Path) -> io::Result<Vec<SourceRef>> {
    let Some(stem) = path.file_stem().and_then(|stem| stem.to_str()) else {
        return Ok(Vec::new());
    };
    let directory = path.parent().unwrap_or_else(|| Path::new("."));
    let mut candidates = Vec::new();
    for entry in std::fs::read_dir(directory)? {
        let candidate = entry?.path();
        if candidate.is_file() && matches_sbi_stem(&candidate, stem) {
            candidates.push(SourceRef::new(candidate.to_string_lossy().into_owned()));
        }
    }
    candidates.sort_by(|left, right| left.0.cmp(&right.0));
    Ok(candidates)
}

fn zip_candidates(archive_path: &Path, entry_name: &str) -> io::Result<Vec<SourceRef>> {
    let disc_path = Path::new(entry_name);
    let Some(stem) = disc_path.file_stem().and_then(|stem| stem.to_str()) else {
        return Ok(Vec::new());
    };
    let directory = disc_path.parent().unwrap_or_else(|| Path::new(""));
    let file = std::fs::File::open(archive_path)?;
    let mut archive = zip::ZipArchive::new(file)
        .map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error))?;
    let mut candidates = Vec::new();
    for index in 0..archive.len() {
        let entry = archive
            .by_index(index)
            .map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error))?;
        if entry.is_dir() {
            continue;
        }
        let candidate = PathBuf::from(entry.name());
        if candidate.parent().unwrap_or_else(|| Path::new("")) == directory
            && matches_sbi_stem(&candidate, stem)
        {
            candidates.push(SourceRef::new(format!(
                "zip:{}#{}",
                archive_path.to_string_lossy(),
                entry.name()
            )));
        }
    }
    candidates.sort_by(|left, right| left.0.cmp(&right.0));
    Ok(candidates)
}

fn matches_sbi_stem(path: &Path, expected_stem: &str) -> bool {
    path.extension()
        .and_then(|extension| extension.to_str())
        .is_some_and(|extension| extension.eq_ignore_ascii_case("sbi"))
        && path
            .file_stem()
            .and_then(|stem| stem.to_str())
            .is_some_and(|stem| stem.eq_ignore_ascii_case(expected_stem))
}

fn validate_sbi(
    source: &SourceRef,
    content: &crate::core::input_content::InputContent,
) -> io::Result<()> {
    if content.size < SBI_HEADER.len() as u64
        || !(content.size - SBI_HEADER.len() as u64).is_multiple_of(SBI_RECORD_SIZE)
    {
        return Err(invalid_sbi(
            source,
            "size does not contain complete 14-byte records",
        ));
    }

    let mut reader = content.handle.open()?;
    let mut header = [0; SBI_HEADER.len()];
    read_exact_at(reader.as_mut(), 0, &mut header)
        .map_err(|_| invalid_sbi(source, "ended before its declared size"))?;
    if &header != SBI_HEADER {
        return Err(invalid_sbi(source, "has an invalid header"));
    }
    let mut offset = SBI_HEADER.len() as u64;
    let mut record = [0; SBI_RECORD_SIZE as usize];
    while offset < content.size {
        read_exact_at(reader.as_mut(), offset, &mut record)
            .map_err(|_| invalid_sbi(source, "ended before its declared size"))?;
        if !record[..3].iter().copied().all(valid_packed_bcd) {
            return Err(invalid_sbi(source, "contains an invalid BCD position"));
        }
        if record[3] != 1 {
            return Err(invalid_sbi(source, "contains a non-Q subchannel record"));
        }
        offset += SBI_RECORD_SIZE;
    }
    Ok(())
}

fn read_exact_at(
    reader: &mut dyn crate::core::reader::Reader,
    offset: u64,
    buffer: &mut [u8],
) -> io::Result<()> {
    let mut read = 0;
    while read < buffer.len() {
        let count = reader.read_at(offset + read as u64, &mut buffer[read..])?;
        if count == 0 {
            return Err(io::Error::new(
                io::ErrorKind::UnexpectedEof,
                "reader ended before requested bytes",
            ));
        }
        read += count;
    }
    Ok(())
}

fn valid_packed_bcd(value: u8) -> bool {
    value >> 4 <= 9 && value & 0x0f <= 9
}

fn invalid_sbi(source: &SourceRef, message: &str) -> io::Error {
    io::Error::new(
        io::ErrorKind::InvalidData,
        format!("SBI sidecar '{source}' {message}"),
    )
}
