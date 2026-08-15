//! user configs store long term ui preferences & customizations

use serde::de::{self, Deserializer, MapAccess, Visitor};
use serde::ser::{SerializeStruct, Serializer};
use serde::{Deserialize, Serialize};
use std::fmt;

// todo: save as config variable
pub const DEFAULT_BPM: f32 = 120.0;

pub struct UserSettings {
    pub instrument_search_paths: Vec<String>,
}

impl Default for UserSettings {
    fn default() -> Self {
        Self {
            instrument_search_paths: Vec::new(),
        }
    }
}

impl Serialize for UserSettings {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut state = serializer.serialize_struct("UserSettings", 1)?;
        state.serialize_field("instrument_search_paths", &self.instrument_search_paths)?;
        state.end()
    }
}

impl<'de> Deserialize<'de> for UserSettings {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct UserSettingsVisitor;

        impl<'de> Visitor<'de> for UserSettingsVisitor {
            type Value = UserSettings;

            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                formatter.write_str("struct UserSettings")
            }

            fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error>
            where
                A: MapAccess<'de>,
            {
                let mut instrument_search_paths = None;

                while let Some(key) = map.next_key::<String>()? {
                    match key.as_str() {
                        "instrument_search_paths" => {
                            if instrument_search_paths.is_some() {
                                return Err(de::Error::duplicate_field("instrument_search_paths"));
                            }
                            instrument_search_paths = Some(map.next_value()?);
                        }
                        _ => {
                            let _: serde::de::IgnoredAny = map.next_value()?;
                        }
                    }
                }

                Ok(UserSettings {
                    instrument_search_paths: instrument_search_paths.unwrap_or_default(),
                })
            }
        }

        deserializer.deserialize_struct(
            "UserSettings",
            &["instrument_search_paths"],
            UserSettingsVisitor,
        )
    }
}

/// get the UserSettings path
pub fn config_path() -> std::path::PathBuf {
    dirs::config_dir()
        .unwrap_or_else(|| std::path::PathBuf::from("."))
        .join("glacier")
        .join("settings.toml")
}

/// get the UserSettings data from disk
pub fn load() -> UserSettings {
    let path = config_path();
    if path.exists() {
        let contents = std::fs::read_to_string(&path).unwrap_or_default();
        toml::from_str(&contents).unwrap_or_default()
    } else {
        UserSettings::default()
    }
}

/// save the UserSettings data to disk
pub fn save(settings: &UserSettings) {
    let path = config_path();
    std::fs::create_dir_all(path.parent().unwrap()).ok();
    let contents = toml::to_string(settings).unwrap();
    std::fs::write(path, contents).ok();
}
