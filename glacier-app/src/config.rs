// config.rs - user configs store long term ui preferences & customizations

use serde::{Deserialize, Serialize};

// todo: save as config variable
pub const DEFAULT_BPM: f32 = 120.0;

#[derive(Serialize, Deserialize, Default)]
pub struct UserSettings {
    pub instrument_search_paths: Vec<String>,
}

// get the UserSettings path
pub fn config_path() -> std::path::PathBuf {
    dirs::config_dir()
        .unwrap_or_else(|| std::path::PathBuf::from("."))
        .join("glacier")
        .join("settings.toml")
}

// get the UserSettings data
pub fn load() -> UserSettings {
    let path = config_path();
    if path.exists() {
        let contents = std::fs::read_to_string(&path).unwrap_or_default();
        toml::from_str(&contents).unwrap_or_default()
    } else {
        UserSettings::default()
    }
}

// save the UserSettings data
pub fn save(settings: &UserSettings) {
    let path = config_path();
    std::fs::create_dir_all(path.parent().unwrap()).ok();
    let contents = toml::to_string(settings).unwrap();
    std::fs::write(path, contents).ok();
}
