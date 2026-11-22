use serde::Deserialize;
use std::collections::HashMap;

#[derive(Deserialize, Clone)]
pub struct Fix {
    pub description: String,
    pub command: String,
}

pub struct Rules {
    fixes: HashMap<String, Vec<Fix>>,
}

impl Rules {
    pub fn load_builtin() -> Rules {
        let text = include_str!("../fixes.toml");
        let fixes: HashMap<String, Vec<Fix>> = toml::from_str(text).expect("fixes.toml is invalid");
        Rules { fixes }
    }

    pub fn for_category(&self, category: &str) -> Vec<Fix> {
        match self.fixes.get(category) {
            Some(list) => list.clone(),
            None => Vec::new(),
        }
    }
}