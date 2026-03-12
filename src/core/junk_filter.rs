use crate::core::virtual_file::VirtualFile;

/// Remove universally unwanted junk files from a discovered file set.
///
/// This is intentionally conservative. Output-specific filtering
/// (for example, whether `.nfo` or cover art should be exposed)
/// belongs in the output/view layer rather than the generic loader.
pub fn filter_universal_junk(files: Vec<VirtualFile>) -> Vec<VirtualFile> {
    files
        .into_iter()
        .filter(|vf| !is_universal_junk(&vf.name))
        .collect()
}

fn is_universal_junk(name: &str) -> bool {
    let lower = name.to_ascii_lowercase();

    if lower.starts_with("__macosx/") {
        return true;
    }

    let file_name = lower.rsplit('/').next().unwrap_or(&lower);

    matches!(file_name, ".ds_store" | "thumbs.db")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_known_junk_files() {
        assert!(is_universal_junk("__MACOSX/game.sfc"));
        assert!(is_universal_junk(".DS_Store"));
        assert!(is_universal_junk("Thumbs.db"));
    }

    #[test]
    fn does_not_reject_sidecar_metadata_files() {
        assert!(!is_universal_junk("readme.txt"));
        assert!(!is_universal_junk("release.nfo"));
        assert!(!is_universal_junk("checksums.sfv"));
        assert!(!is_universal_junk("notes.md"));
        assert!(!is_universal_junk("cover.jpg"));
        assert!(!is_universal_junk("screenshot.png"));
    }

    #[test]
    fn does_not_reject_likely_payload_files() {
        assert!(!is_universal_junk("game.sfc"));
        assert!(!is_universal_junk("disc1.bin"));
        assert!(!is_universal_junk("image.iso"));
        assert!(!is_universal_junk("archive/game.chd"));
    }
}
