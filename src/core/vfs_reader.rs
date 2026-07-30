use std::io;

use crate::core::reader::Reader;
use crate::core::source_resolver::open_source_ref;
use crate::core::vfs::{FileBacking, VfsFile};
use crate::readers::inline_reader::InlineReader;

pub fn open_vfs_file(file: &VfsFile) -> Result<Box<dyn Reader>, io::Error> {
    match &file.backing {
        FileBacking::Inline(contents) => Ok(Box::new(InlineReader::new(contents.clone()))),
        FileBacking::Source(source) => open_source_ref(source),
        FileBacking::Reader(handle) => handle.open(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::reader_handle::ReaderHandle;
    use crate::core::source::SourceRef;
    use crate::core::vfs::VfsFile;
    use std::fs;
    use std::io::Write;

    #[test]
    fn opens_inline_backed_vfs_file() {
        let file = VfsFile::inline("game.m3u", b"disc1.cue\ndisc2.cue\n".to_vec());

        let mut reader = open_vfs_file(&file).unwrap();
        let mut buf = vec![0; file.size as usize];
        let bytes = reader.read_at(0, &mut buf).unwrap();

        assert_eq!(bytes, file.size as usize);
        assert_eq!(&buf, b"disc1.cue\ndisc2.cue\n");
    }

    #[test]
    fn opens_live_reader_backed_vfs_file_at_arbitrary_offsets() {
        let handle = ReaderHandle::new("test:logical-disc", || {
            Ok(Box::new(InlineReader::new(b"logical disc bytes".to_vec())))
        });
        let file = VfsFile::reader_backed("game.iso", 18, handle);

        let mut reader = open_vfs_file(&file).unwrap();
        let mut buf = vec![0; 4];
        let bytes = reader.read_at(8, &mut buf).unwrap();

        assert_eq!(bytes, 4);
        assert_eq!(&buf, b"disc");
    }

    #[test]
    fn opens_filesystem_backed_vfs_file() {
        let temp_dir = tempfile::tempdir().unwrap();
        let path = temp_dir.path().join("readme.txt");
        fs::write(&path, b"hello world").unwrap();

        let file = VfsFile::source_backed(
            "readme.txt",
            11,
            SourceRef::new(path.to_string_lossy().into_owned()),
        );

        let mut reader = open_vfs_file(&file).unwrap();
        let mut buf = vec![0; 11];
        let bytes = reader.read_at(0, &mut buf).unwrap();

        assert_eq!(bytes, 11);
        assert_eq!(&buf, b"hello world");
    }

    #[test]
    fn opens_zip_backed_vfs_file() {
        use std::fs::File;

        let temp_dir = tempfile::tempdir().unwrap();
        let zip_path = temp_dir.path().join("library.zip");

        {
            let file = File::create(&zip_path).unwrap();
            let mut zip = zip::ZipWriter::new(file);
            let options: zip::write::SimpleFileOptions = zip::write::SimpleFileOptions::default();

            zip.start_file("docs/readme.txt", options).unwrap();
            zip.write_all(b"zip hello").unwrap();
            zip.finish().unwrap();
        }

        let file = VfsFile::source_backed(
            "docs/readme.txt",
            9,
            SourceRef::new(format!(
                "zip:{}#docs/readme.txt",
                zip_path.to_string_lossy()
            )),
        );

        let mut reader = open_vfs_file(&file).unwrap();
        let mut buf = vec![0; 9];
        let bytes = reader.read_at(0, &mut buf).unwrap();

        assert_eq!(bytes, 9);
        assert_eq!(&buf, b"zip hello");
    }

    #[test]
    fn reads_inline_backed_vfs_file_from_offset() {
        let file = VfsFile::inline("game.m3u", b"disc1.cue\ndisc2.cue\n".to_vec());

        let mut reader = open_vfs_file(&file).unwrap();
        let mut buf = vec![0; 8];
        let bytes = reader.read_at(5, &mut buf).unwrap();

        assert_eq!(bytes, 8);
        assert_eq!(&buf, b".cue\ndis");
    }

    #[test]
    fn reads_zip_backed_vfs_file_from_offset() {
        use std::fs::File;

        let temp_dir = tempfile::tempdir().unwrap();
        let zip_path = temp_dir.path().join("library.zip");

        {
            let file = File::create(&zip_path).unwrap();
            let mut zip = zip::ZipWriter::new(file);
            let options: zip::write::SimpleFileOptions = zip::write::SimpleFileOptions::default();

            zip.start_file("docs/readme.txt", options).unwrap();
            zip.write_all(b"zip hello").unwrap();
            zip.finish().unwrap();
        }

        let file = VfsFile::source_backed(
            "docs/readme.txt",
            9,
            SourceRef::new(format!(
                "zip:{}#docs/readme.txt",
                zip_path.to_string_lossy()
            )),
        );

        let mut reader = open_vfs_file(&file).unwrap();
        let mut buf = vec![0; 4];
        let bytes = reader.read_at(4, &mut buf).unwrap();

        assert_eq!(bytes, 4);
        assert_eq!(&buf, b"hell");
    }
}
