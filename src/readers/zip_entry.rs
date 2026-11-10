use std::io::{self, Read, Seek};

/// Source identities contain decoded names, so resolve only an exact decoded
/// match. Raw-byte precedence or taking the first decoded match would
/// silently alias distinct members to one identity.
/// This scans metadata only; it never opens or decompresses entry payloads.
pub(crate) fn resolve_entry_index<R: Read + Seek>(
    archive: &zip::ZipArchive<R>,
    name: &str,
) -> io::Result<usize> {
    let mut found = None;
    for (index, decoded) in archive.file_names().enumerate() {
        let decoded = decoded.map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error))?;
        if decoded == name {
            if found.is_some() {
                return Err(ambiguous_name(name));
            }
            found = Some(index);
        }
    }
    found.ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::NotFound,
            format!("ZIP entry '{name}' not found"),
        )
    })
}

pub(crate) fn ambiguous_name(name: &str) -> io::Error {
    io::Error::new(
        io::ErrorKind::InvalidData,
        format!("ambiguous decoded ZIP entry name '{name}'; rename the conflicting members"),
    )
}
