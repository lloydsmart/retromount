use std::collections::HashSet;
use std::fs;
use std::io::{Error, ErrorKind};
use std::path::Path;

use crate::core::cue::parse_cue;
use crate::core::input_handler::InputHandler;
use crate::core::input_registry::InputRegistry;
use crate::core::reader_registry::ReaderRegistry;
use crate::core::virtual_file::VirtualFile;

/// Input handler for CUE sheets.
///
/// This implementation parses FILE/TRACK metadata from the cue sheet,
/// preserves cue order, resolves referenced files relative to the cue
/// file, and exposes each unique referenced file as a VirtualFile.
///
/// Later versions can build richer disc/track models on top of this.
pub struct CueInputHandler {
    reader_registry: ReaderRegistry,
}

impl CueInputHandler {
    pub fn new(reader_registry: ReaderRegistry) -> Self {
        Self { reader_registry }
    }
}

impl InputHandler for CueInputHandler {
    fn supports(&self, path: &Path) -> bool {
        path.is_file()
            && path
                .extension()
                .is_some_and(|ext| ext.eq_ignore_ascii_case("cue"))
    }

    fn discover(
        &self,
        _registry: &InputRegistry,
        path: &Path,
    ) -> std::io::Result<Vec<VirtualFile>> {
        let cue_text = fs::read_to_string(path)?;
        let cue_dir = path.parent().unwrap_or_else(|| Path::new("."));
        let parsed = parse_cue(&cue_text);

        let mut files = Vec::new();
        let mut seen = HashSet::new();

        for file_entry in parsed {
            let referenced_path = cue_dir.join(&file_entry.path);

            if !seen.insert(referenced_path.clone()) {
                continue;
            }

            if !referenced_path.is_file() {
                return Err(Error::new(
                    ErrorKind::NotFound,
                    format!(
                        "CUE file '{}' references missing file '{}'",
                        path.display(),
                        referenced_path.display()
                    ),
                ));
            }

            let reader = self.reader_registry.open(&referenced_path)?;
            let len = reader.len();

            let name = referenced_path
                .file_name()
                .map(|n| n.to_string_lossy().into_owned())
                .unwrap_or_else(|| referenced_path.display().to_string());

            files.push(VirtualFile::new(name, len, referenced_path, reader));
        }

        Ok(files)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn cue_input_handler_discovers_referenced_bin_files() {
        let dir = TempDir::new().expect("failed to create temp dir");

        let cue_path = dir.path().join("game.cue");
        let bin_path = dir.path().join("track01.bin");

        fs::write(
            &cue_path,
            r#"
FILE "track01.bin" BINARY
  TRACK 01 MODE1/2352
    INDEX 01 00:00:00
"#,
        )
        .expect("failed to write cue file");

        fs::write(&bin_path, b"fake-bin-data").expect("failed to write bin file");

        let handler = CueInputHandler::new(ReaderRegistry::default());
        let registry = InputRegistry::default();

        let files = handler
            .discover(&registry, &cue_path)
            .expect("cue discovery failed");

        assert_eq!(files.len(), 1);
        assert_eq!(files[0].name, "track01.bin");
        assert_eq!(files[0].size, 13);
        assert_eq!(files[0].origin, bin_path);
    }

    #[test]
    fn cue_input_handler_deduplicates_repeated_file_entries() {
        let dir = TempDir::new().expect("failed to create temp dir");

        let cue_path = dir.path().join("game.cue");
        let bin_path = dir.path().join("track01.bin");

        fs::write(
            &cue_path,
            r#"
FILE "track01.bin" BINARY
  TRACK 01 MODE1/2352
    INDEX 01 00:00:00
FILE "track01.bin" BINARY
  TRACK 02 AUDIO
    INDEX 01 00:00:00
"#,
        )
        .expect("failed to write cue file");

        fs::write(&bin_path, b"fake-bin-data").expect("failed to write bin file");

        let handler = CueInputHandler::new(ReaderRegistry::default());
        let registry = InputRegistry::default();

        let files = handler
            .discover(&registry, &cue_path)
            .expect("cue discovery failed");

        assert_eq!(files.len(), 1);
        assert_eq!(files[0].name, "track01.bin");
        assert_eq!(files[0].origin, bin_path);
    }

    #[test]
    fn cue_input_handler_preserves_cue_file_order() {
        let dir = TempDir::new().expect("failed to create temp dir");

        let cue_path = dir.path().join("game.cue");
        let track01 = dir.path().join("track01.bin");
        let track02 = dir.path().join("track02.bin");

        fs::write(
            &cue_path,
            r#"
FILE "track02.bin" BINARY
  TRACK 02 AUDIO
    INDEX 01 00:00:00
FILE "track01.bin" BINARY
  TRACK 01 MODE1/2352
    INDEX 01 00:00:00
"#,
        )
        .expect("failed to write cue file");

        fs::write(&track01, b"track01").expect("failed to write track01");
        fs::write(&track02, b"track02").expect("failed to write track02");

        let handler = CueInputHandler::new(ReaderRegistry::default());
        let registry = InputRegistry::default();

        let files = handler
            .discover(&registry, &cue_path)
            .expect("cue discovery failed");

        assert_eq!(files.len(), 2);
        assert_eq!(files[0].name, "track02.bin");
        assert_eq!(files[1].name, "track01.bin");
    }

    #[test]
    fn cue_input_handler_returns_clear_error_for_missing_referenced_file() {
        let dir = TempDir::new().expect("failed to create temp dir");

        let cue_path = dir.path().join("game.cue");

        fs::write(
            &cue_path,
            r#"
FILE "missing.bin" BINARY
  TRACK 01 MODE1/2352
    INDEX 01 00:00:00
"#,
        )
        .expect("failed to write cue file");

        let handler = CueInputHandler::new(ReaderRegistry::default());
        let registry = InputRegistry::default();

        let result = handler.discover(&registry, &cue_path);

        match result {
            Ok(_) => panic!("expected missing-file error"),
            Err(err) => {
                assert_eq!(err.kind(), ErrorKind::NotFound);
                assert!(err.to_string().contains("references missing file"));
                assert!(err.to_string().contains("missing.bin"));
            }
        }
    }
}
