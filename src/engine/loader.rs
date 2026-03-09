use std::path::Path;

use crate::core::game_image::GameImage;
use crate::error::RetromountError;
use crate::plugin::registry::PluginRegistry;

pub struct Loader<'a> {
    registry: &'a PluginRegistry,
}

impl<'a> Loader<'a> {
    pub fn new(registry: &'a PluginRegistry) -> Self {
        Self { registry }
    }

    pub fn load_path(&self, path: &Path) -> Result<GameImage, RetromountError> {
        let plugin = self
            .registry
            .detect_input(path)
            .ok_or(RetromountError::UnsupportedFormat)?;

        plugin.load(path)
    }
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use crate::core::disc::Disc;
    use crate::core::game_image::GameImage;
    use crate::core::platform::Platform;
    use crate::error::RetromountError;
    use crate::plugin::input::InputPlugin;
    use crate::plugin::registry::PluginRegistry;

    use super::Loader;

    struct TestInputPlugin;

    impl InputPlugin for TestInputPlugin {
        fn name(&self) -> &'static str {
            "test-input"
        }

        fn detect(&self, path: &Path) -> bool {
            path.extension().is_some_and(|ext| ext == "test")
        }

        fn load(&self, _path: &Path) -> Result<GameImage, RetromountError> {
            Ok(GameImage {
                id: "test-game".to_string(),
                title: "Test Game".to_string(),
                platform: Platform::GenericCD,
                discs: vec![Disc {
                    number: 1,
                    tracks: vec![],
                }],
            })
        }
    }

    #[test]
    fn loads_game_image_using_detected_plugin() {
        let mut registry = PluginRegistry::new();
        registry.register_input(TestInputPlugin);

        let loader = Loader::new(&registry);
        let image = loader.load_path(Path::new("game.test")).unwrap();

        assert_eq!(image.id, "test-game");
        assert_eq!(image.title, "Test Game");
        assert_eq!(image.platform, Platform::GenericCD);
        assert_eq!(image.discs.len(), 1);
    }

    #[test]
    fn returns_unsupported_format_when_no_plugin_matches() {
        let registry = PluginRegistry::new();
        let loader = Loader::new(&registry);

        let result = loader.load_path(Path::new("game.unknown"));

        assert!(matches!(result, Err(RetromountError::UnsupportedFormat)));
    }
}