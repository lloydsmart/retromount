pub mod directory_input;
pub mod file_input;
pub mod zip_input;

use crate::core::input_registry::InputRegistry;
use crate::core::reader_registry::ReaderRegistry;

/// Register all built-in input handlers.
///
/// Registration order matters:
/// more specific handlers must be registered before more general ones.
pub fn register_builtin_inputs(registry: &mut InputRegistry) {
    registry.register(Box::new(directory_input::DirectoryInputHandler));
    registry.register(Box::new(zip_input::ZipInputHandler));
    registry.register(Box::new(file_input::FileInputHandler::new(
        ReaderRegistry::default(),
    )));
}
