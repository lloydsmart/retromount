use std::fmt;
use std::io;

use serde::Serialize;

use crate::core::reader::Reader;
use crate::core::reader_handle::ReaderHandle;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum InputAccess {
    Sequential,
    RandomAccess,
}

/// Opaque, clonable access to the encoded bytes of one input object.
#[derive(Clone, PartialEq, Eq, Serialize)]
pub struct InputContent {
    pub size: u64,
    pub access: InputAccess,
    pub handle: ReaderHandle,
}

impl InputContent {
    pub fn new(size: u64, access: InputAccess, handle: ReaderHandle) -> Self {
        Self {
            size,
            access,
            handle,
        }
    }

    pub fn open(&self) -> io::Result<Box<dyn Reader>> {
        self.handle.open()
    }

    pub fn open_random_access(&self) -> io::Result<Box<dyn Reader>> {
        if self.access != InputAccess::RandomAccess {
            return Err(io::Error::new(
                io::ErrorKind::Unsupported,
                "input does not provide efficient random access",
            ));
        }

        self.open()
    }
}

impl fmt::Debug for InputContent {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("InputContent")
            .field("size", &self.size)
            .field("access", &self.access)
            .field("handle", &self.handle)
            .finish()
    }
}
