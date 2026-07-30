use std::fmt;
use std::io;
use std::sync::Arc;

use serde::ser::SerializeStruct;
use serde::{Serialize, Serializer};

use crate::core::reader::Reader;

type ReaderOpener = dyn Fn() -> io::Result<Box<dyn Reader>> + Send + Sync;

/// A clonable, opaque handle that can open a live random-access reader.
///
/// Runtime behavior stays in process while debug/inspection output exposes
/// only the stable identifier.
#[derive(Clone)]
pub struct ReaderHandle {
    id: Arc<str>,
    open: Arc<ReaderOpener>,
}

impl ReaderHandle {
    pub fn new(
        id: impl Into<Arc<str>>,
        open: impl Fn() -> io::Result<Box<dyn Reader>> + Send + Sync + 'static,
    ) -> Self {
        Self {
            id: id.into(),
            open: Arc::new(open),
        }
    }

    pub fn id(&self) -> &str {
        &self.id
    }

    pub fn open(&self) -> io::Result<Box<dyn Reader>> {
        (self.open)()
    }
}

impl fmt::Debug for ReaderHandle {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("ReaderHandle")
            .field("id", &self.id)
            .finish_non_exhaustive()
    }
}

impl PartialEq for ReaderHandle {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}

impl Eq for ReaderHandle {}

impl Serialize for ReaderHandle {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut state = serializer.serialize_struct("ReaderHandle", 1)?;
        state.serialize_field("id", self.id())?;
        state.end()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::readers::inline_reader::InlineReader;

    #[test]
    fn opens_independent_random_access_readers() {
        let handle = ReaderHandle::new("test:disc", || {
            Ok(Box::new(InlineReader::new(b"disc bytes".to_vec())))
        });

        let mut first = handle.open().unwrap();
        let mut second = handle.open().unwrap();
        let mut first_bytes = [0; 4];
        let mut second_bytes = [0; 5];

        first.read_at(5, &mut first_bytes).unwrap();
        second.read_at(0, &mut second_bytes).unwrap();

        assert_eq!(&first_bytes, b"byte");
        assert_eq!(&second_bytes, b"disc ");
    }

    #[test]
    fn serializes_only_the_stable_identifier() {
        let handle = ReaderHandle::new("test:disc", || Ok(Box::new(InlineReader::new(Vec::new()))));

        assert_eq!(
            serde_json::to_value(handle).unwrap(),
            serde_json::json!({ "id": "test:disc" })
        );
    }
}
