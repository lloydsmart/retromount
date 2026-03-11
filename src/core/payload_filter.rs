use crate::core::virtual_file::VirtualFile;

pub fn filter_payload_files(files: Vec<VirtualFile>) -> Vec<VirtualFile> {
    files.into_iter()
        .filter(|vf| is_payload_file(&vf.name))
        .collect()
}

pub fn is_payload_file(name: &str) -> bool {
    let lower = name.to_ascii_lowercase();

    if lower.starts_with("__macosx/") {
        return false;
    }

    let file_name = lower.rsplit('/').next().unwrap_or(&lower);

    if matches!(file_name, ".ds_store" | "thumbs.db") {
        return false;
    }

    let ext = std::path::Path::new(file_name)
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("");

    !matches!(ext, "txt" | "nfo" | "sfv" | "md")
}