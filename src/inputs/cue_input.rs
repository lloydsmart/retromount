use std::fs;
use std::path::{Path, PathBuf};

use crate::core::input_handler::InputHandler;
use crate::core::input_registry::InputRegistry;
use crate::core::reader_registry::ReaderRegistry;
use crate::core::virtual_file::VirtualFile;

/// Input handler for CUE sheets.
///
/// The initial implementation is intentionally simple:
/// it parses `FILE` lines from the cue sheet and exposes each referenced
/// file as a VirtualFile using the normal reader pipeline.
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

        let mut files = Vec::new();

        for referenced_name in parse_cue_file_entries(&cue_text) {
            let referenced_path = cue_dir.join(&referenced_name);

            let reader = self.reader_registry.open(&referenced_path)?;
            let size = reader.size();

            let name = referenced_path
                .file_name()
                .map(|n| n.to_string_lossy().into_owned())
                .unwrap_or_else(|| referenced_path.display().to_string());

            files.push(VirtualFile::new(name, size, referenced_path, reader));
        }

        Ok(files)
    }
}

fn parse_cue_file_entries(cue_text: &str) -> Vec<PathBuf> {
    cue_text
        .lines()
        .filter_map(parse_cue_file_line)
        .map(PathBuf::from)
        .collect()
}

fn parse_cue_file_line(line: &str) -> Option<String> {
    let trimmed = line.trim_start();

    if !trimmed
        .get(0..4)
        .is_some_and(|prefix| prefix.eq_ignore_ascii_case("FILE"))
    {
        return None;
    }

    let rest = trimmed.get(4..)?.trim_start();

    if let Some(quoted) = rest.strip_prefix('"') {
        let end = quoted.find('"')?;
        return Some(quoted[..end].to_string());
    }

    let file_name = rest.split_whitespace().next()?;
    Some(file_name.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn parses_quoted_file_entries() {
        let cue = r#"
            FILE "Track 01.bin" BINARY
            TRACK 01 MODE1/2352
            INDEX 01 00:00:00
        "#;

        let files = parse_cue_file_entries(cue);

        assert_eq!(files, vec![PathBuf::from("Track 01.bin")]);
    }

    #[test]
    fn parses_unquoted_file_entries() {
        let cue = r#"
            FILE track01.bin BINARY
            TRACK 01 MODE1/2352
            INDEX 01 00:00:00
        "#;

        let files = parse_cue_file_entries(cue);

        assert_eq!(files, vec![PathBuf::from("track01.bin")]);
    }

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
}
