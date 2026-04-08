use std::collections::HashMap;

use crate::output::encode::OutputEncoder;

pub struct EncoderRegistry {
    encoders: HashMap<String, Box<dyn Fn() -> Box<dyn OutputEncoder>>>,
}

impl EncoderRegistry {
    pub fn new() -> Self {
        Self {
            encoders: HashMap::new(),
        }
    }

    pub fn register<F>(&mut self, name: &str, factory: F)
    where
        F: Fn() -> Box<dyn OutputEncoder> + 'static,
    {
        self.encoders.insert(name.to_string(), Box::new(factory));
    }

    pub fn get(&self, name: &str) -> Option<Box<dyn OutputEncoder>> {
        self.encoders.get(name).map(|factory| factory())
    }

    pub fn names(&self) -> Vec<&str> {
        let mut names: Vec<&str> = self.encoders.keys().map(|k| k.as_str()).collect();
        names.sort_unstable();
        names
    }
}

impl Default for EncoderRegistry {
    fn default() -> Self {
        Self::new()
    }
}
