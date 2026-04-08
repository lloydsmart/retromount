use std::collections::HashMap;

use crate::output::present::OutputPresenter;

pub struct PresenterRegistry {
    presenters: HashMap<String, Box<dyn Fn() -> Box<dyn OutputPresenter>>>,
}

impl PresenterRegistry {
    pub fn new() -> Self {
        Self {
            presenters: HashMap::new(),
        }
    }

    pub fn register<F>(&mut self, name: &str, factory: F)
    where
        F: Fn() -> Box<dyn OutputPresenter> + 'static,
    {
        self.presenters.insert(name.to_string(), Box::new(factory));
    }

    pub fn get(&self, name: &str) -> Option<Box<dyn OutputPresenter>> {
        self.presenters.get(name).map(|factory| factory())
    }

    pub fn names(&self) -> Vec<&str> {
        let mut names: Vec<&str> = self.presenters.keys().map(|k| k.as_str()).collect();
        names.sort_unstable();
        names
    }
}

impl Default for PresenterRegistry {
    fn default() -> Self {
        Self::new()
    }
}
