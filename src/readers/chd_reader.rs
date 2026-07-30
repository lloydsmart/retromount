use std::collections::VecDeque;
use std::fs::File;
use std::io;
use std::path::Path;

use chd::Chd;

use crate::core::reader::Reader;

const DEFAULT_CACHE_HUNKS: usize = 4;

trait HunkSource: Send + Sync {
    fn hunk_size(&self) -> u32;
    fn logical_bytes(&self) -> u64;
    fn read_hunk(&mut self, index: u32, output: &mut [u8]) -> io::Result<()>;
}

struct ChdHunkSource {
    chd: Chd<File>,
    compressed: Vec<u8>,
}

impl HunkSource for ChdHunkSource {
    fn hunk_size(&self) -> u32 {
        self.chd.header().hunk_size()
    }

    fn logical_bytes(&self) -> u64 {
        self.chd.header().logical_bytes()
    }

    fn read_hunk(&mut self, index: u32, output: &mut [u8]) -> io::Result<()> {
        self.chd
            .hunk(index)
            .and_then(|mut hunk| hunk.read_hunk_in(&mut self.compressed, output))
            .map(|_| ())
            .map_err(|error| io::Error::other(format!("failed to read CHD hunk {index}: {error}")))
    }
}

/// Provides bounded, live random access to the logical byte stream of a CHD.
pub struct ChdReader {
    source: Box<dyn HunkSource>,
    hunk_size: u64,
    logical_bytes: u64,
    cache_capacity: usize,
    cache: VecDeque<(u32, Vec<u8>)>,
}

impl ChdReader {
    pub fn open(path: &Path) -> io::Result<Self> {
        let file = File::open(path)?;
        let chd = Chd::open(file, None)
            .map_err(|error| io::Error::other(format!("failed to open CHD: {error}")))?;

        Self::from_source(
            Box::new(ChdHunkSource {
                chd,
                compressed: Vec::new(),
            }),
            DEFAULT_CACHE_HUNKS,
        )
    }

    fn from_source(source: Box<dyn HunkSource>, cache_capacity: usize) -> io::Result<Self> {
        let hunk_size = u64::from(source.hunk_size());
        if hunk_size == 0 || cache_capacity == 0 {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "CHD hunk size and cache capacity must be non-zero",
            ));
        }

        let logical_bytes = source.logical_bytes();
        Ok(Self {
            source,
            hunk_size,
            logical_bytes,
            cache_capacity,
            cache: VecDeque::with_capacity(cache_capacity),
        })
    }

    fn ensure_hunk(&mut self, index: u32) -> io::Result<()> {
        if let Some(position) = self
            .cache
            .iter()
            .position(|(cached_index, _)| *cached_index == index)
        {
            let cached = self.cache.remove(position).expect("cache entry exists");
            self.cache.push_back(cached);
            return Ok(());
        }

        let mut bytes = vec![0; self.hunk_size as usize];
        self.source.read_hunk(index, &mut bytes)?;

        if self.cache.len() == self.cache_capacity {
            self.cache.pop_front();
        }
        self.cache.push_back((index, bytes));
        Ok(())
    }
}

impl Reader for ChdReader {
    fn read_at(&mut self, offset: u64, buffer: &mut [u8]) -> io::Result<usize> {
        if offset >= self.logical_bytes || buffer.is_empty() {
            return Ok(0);
        }

        let requested = (self.logical_bytes - offset).min(buffer.len() as u64) as usize;
        let mut copied = 0;

        while copied < requested {
            let logical_offset = offset + copied as u64;
            let hunk_index = u32::try_from(logical_offset / self.hunk_size).map_err(|_| {
                io::Error::new(io::ErrorKind::InvalidData, "CHD hunk index overflow")
            })?;
            let offset_in_hunk = (logical_offset % self.hunk_size) as usize;
            self.ensure_hunk(hunk_index)?;

            let (_, hunk) = self.cache.back().expect("requested hunk is cached");
            let available = hunk.len() - offset_in_hunk;
            let count = available.min(requested - copied);
            buffer[copied..copied + count]
                .copy_from_slice(&hunk[offset_in_hunk..offset_in_hunk + count]);
            copied += count;
        }

        Ok(copied)
    }

    fn len(&self) -> u64 {
        self.logical_bytes
    }
}

#[cfg(test)]
mod tests {
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::Arc;

    use super::*;

    struct FakeHunkSource {
        bytes: Vec<u8>,
        hunk_size: u32,
        logical_bytes: u64,
        reads: Arc<AtomicUsize>,
    }

    impl HunkSource for FakeHunkSource {
        fn hunk_size(&self) -> u32 {
            self.hunk_size
        }

        fn logical_bytes(&self) -> u64 {
            self.logical_bytes
        }

        fn read_hunk(&mut self, index: u32, output: &mut [u8]) -> io::Result<()> {
            self.reads.fetch_add(1, Ordering::SeqCst);
            let start = index as usize * self.hunk_size as usize;
            let available = self.bytes.len().saturating_sub(start).min(output.len());
            output[..available].copy_from_slice(&self.bytes[start..start + available]);
            Ok(())
        }
    }

    fn reader(cache_capacity: usize, reads: Arc<AtomicUsize>) -> ChdReader {
        ChdReader::from_source(
            Box::new(FakeHunkSource {
                bytes: (0..24).collect(),
                hunk_size: 8,
                logical_bytes: 20,
                reads,
            }),
            cache_capacity,
        )
        .unwrap()
    }

    #[test]
    fn reads_across_hunk_boundaries_and_clamps_at_logical_end() {
        let mut reader = reader(2, Arc::new(AtomicUsize::new(0)));
        let mut across = [0; 10];
        let mut tail = [0; 8];

        assert_eq!(reader.read_at(6, &mut across).unwrap(), 10);
        assert_eq!(across, [6, 7, 8, 9, 10, 11, 12, 13, 14, 15]);
        assert_eq!(reader.read_at(18, &mut tail).unwrap(), 2);
        assert_eq!(&tail[..2], &[18, 19]);
    }

    #[test]
    fn reuses_cached_hunks_and_evicts_to_the_configured_bound() {
        let reads = Arc::new(AtomicUsize::new(0));
        let mut reader = reader(1, reads.clone());
        let mut byte = [0; 1];

        reader.read_at(0, &mut byte).unwrap();
        reader.read_at(1, &mut byte).unwrap();
        assert_eq!(reads.load(Ordering::SeqCst), 1);

        reader.read_at(8, &mut byte).unwrap();
        reader.read_at(0, &mut byte).unwrap();
        assert_eq!(reads.load(Ordering::SeqCst), 3);
        assert_eq!(reader.cache.len(), 1);
    }

    #[test]
    fn propagates_hunk_decode_failures_without_partial_success() {
        struct FailingHunkSource;

        impl HunkSource for FailingHunkSource {
            fn hunk_size(&self) -> u32 {
                8
            }

            fn logical_bytes(&self) -> u64 {
                8
            }

            fn read_hunk(&mut self, index: u32, _output: &mut [u8]) -> io::Result<()> {
                Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    format!("failed to decompress CHD hunk {index}"),
                ))
            }
        }

        let mut reader = ChdReader::from_source(Box::new(FailingHunkSource), 1).unwrap();
        let mut output = [0; 4];
        let error = reader.read_at(0, &mut output).unwrap_err();

        assert_eq!(error.kind(), io::ErrorKind::InvalidData);
        assert!(error.to_string().contains("decompress"));
        assert!(reader.cache.is_empty());
    }
}
