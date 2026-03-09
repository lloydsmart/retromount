use std::path::Path;

use crate::core::game_image::GameImage;
use crate::plugin::input::InputPlugin;
use crate::plugin::output::OutputPlugin;

#[derive(Default)]
pub struct PluginRegistry {
    input_plugins: Vec<Box<dyn InputPlugin>>,
    output_plugins: Vec<Box<dyn OutputPlugin>>,
}

impl PluginRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn register_input<P>(&mut self, plugin: P)
    where
        P: InputPlugin + 'static,
    {
        self.input_plugins.push(Box::new(plugin));
    }

    pub fn register_output<P>(&mut self, plugin: P)
    where
        P: OutputPlugin + 'static,
    {
        self.output_plugins.push(Box::new(plugin));
    }

    pub fn detect_input(&self, path: &Path) -> Option<&dyn InputPlugin> {
        self.input_plugins
            .iter()
            .find(|plugin| plugin.detect(path))
            .map(Box::as_ref)
    }

    pub fn supported_outputs(&self, image: &GameImage) -> Vec<&dyn OutputPlugin> {
        self.output_plugins
            .iter()
            .filter(|plugin| plugin.supports(image))
            .map(Box::as_ref)
            .collect()
    }

    pub fn input_plugins(&self) -> &[Box<dyn InputPlugin>] {
        &self.input_plugins
    }

    pub fn output_plugins(&self) -> &[Box<dyn OutputPlugin>] {
        &self.output_plugins
    }
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use crate::core::disc::Disc;
    use crate::core::game_image::GameImage;
    use crate::core::platform::Platform;
    use crate::plugin::input::InputPlugin;
    use crate::plugin::output::OutputPlugin;

    use super::PluginRegistry;

    struct TestInputPlugin;

    impl InputPlugin for TestInputPlugin {
        fn name(&self) -> &'static str {
            "test-input"
        }

        fn detect(&self, path: &Path) -> bool {
            path.extension().is_some_and(|ext| ext == "test")
        }

        fn load(&self, _path: &Path) -> Result<GameImage, String> {
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

    struct TestOutputPlugin;

    impl OutputPlugin for TestOutputPlugin {
        fn name(&self) -> &'static str {
            "test-output"
        }

        fn supports(&self, image: &GameImage) -> bool {
            image.platform == Platform::GenericCD
        }
    }

    #[test]
    fn registers_and_detects_input_plugin() {
        let mut registry = PluginRegistry::new();
        registry.register_input(TestInputPlugin);

        let plugin = registry.detect_input(Path::new("game.test"));

        assert!(plugin.is_some());
        assert_eq!(plugin.unwrap().name(), "test-input");
    }

    #[test]
    fn returns_supported_output_plugins() {
        let mut registry = PluginRegistry::new();
        registry.register_output(TestOutputPlugin);

        let image = GameImage {
            id: "test-game".to_string(),
            title: "Test Game".to_string(),
            platform: Platform::GenericCD,
            discs: vec![Disc {
                number: 1,
                tracks: vec![],
            }],
        };

        let outputs = registry.supported_outputs(&image);

        assert_eq!(outputs.len(), 1);
        assert_eq!(outputs[0].name(), "test-output");
    }
}