use std::fs;
use super::config::Config;

pub fn load_config(path: &str) -> Config {
    let text = fs::read_to_string(path)
        .expect("Failed to read network yaml file");
    let yaml = serde_yaml::from_str(&text)
        .expect("Failed to parse network yaml file");
    yaml
}
