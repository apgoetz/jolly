// contains logic for parsing jolly config file.
// a config file consists of settings and a store
// settings are parameters for the program
// store represents the links that are stored in jolly

use crate::{error::Error, settings::Settings, store::Store};
use serde::Deserialize;
use std::{fs, path};
use toml;

pub const LOGFILE_NAME: &str = "jolly.toml";

// helper enum to allow decoding a scalar into a single vec
// original hint from here:
// https://github.com/Mingun/ksc-rs/blob/8532f701e660b07b6d2c74963fdc0490be4fae4b/src/parser.rs#L18-L42
// (MIT LICENSE)
#[derive(Clone, Debug, Deserialize, PartialEq)]
#[serde(untagged)]
enum OneOrMany<T> {
    /// Single value
    One(T),
    /// Array of values
    Vec(Vec<T>),
}
impl<T> From<OneOrMany<T>> for Vec<T> {
    fn from(from: OneOrMany<T>) -> Self {
        match from {
            OneOrMany::One(val) => vec![val],
            OneOrMany::Vec(vec) => vec,
        }
    }
}

pub fn one_or_many<'de, T: Deserialize<'de>, D: serde::Deserializer<'de>>(
    d: D,
) -> Result<Vec<T>, D::Error> {
    OneOrMany::deserialize(d).map(Vec::from)
}

// represents the data that is loaded from the main configuration file
// it will always have some settings internally, even if the config
// file is not found.  if there is an error parsing the config, a
// default value for settings will be used so at least a window will
// show
#[derive(Debug)]
pub struct Config {
    pub settings: Settings,
    pub store: Result<Store, Error>,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            settings: Settings::default(),
            store: Err(Error::CustomError("".to_string())),
        }
    }
}

impl Config {
    pub fn load() -> Self {
        match get_logfile().map(load_path) {
            Ok(config) => config.unwrap_or_else(|e| Self {
                settings: Settings::default(),
                store: Err(e),
            }),
            Err(e) => Self {
                settings: Settings::default(),
                store: Err(e),
            },
        }
    }
}

fn get_logfile() -> Result<path::PathBuf, Error> {
    let local_path = path::Path::new(LOGFILE_NAME);
    if local_path.exists() {
        return Ok(local_path.to_path_buf());
    }

    let config_dir = dirs::config_dir().ok_or(Error::CustomError(
        "Cannot Determine Config Dir".to_string(),
    ))?;
    let config_path = config_dir.join(LOGFILE_NAME);
    if config_path.exists() {
        Ok(config_path)
    } else {
        Err(Error::CustomError(format!("Cannot find {}", LOGFILE_NAME)))
    }
}

pub fn load_path<P: AsRef<path::Path>>(path: P) -> Result<Config, Error> {
    let txt = fs::read_to_string(path).map_err(Error::IoError)?;
    load_txt(&txt)
}

fn load_txt(txt: &str) -> Result<Config, Error> {
    let value: toml::Value = toml::from_str(txt).map_err(|e| Error::ParseError(e.to_string()))?;

    let mut parsed_config = match value {
        toml::Value::Table(t) => t,
        _ => return Err(Error::ParseError("entry is not a Table".to_string())),
    };

    // if we have a settings entry use it, otherwise deserialize something empty and rely on serde defaults

    let mut settings = match parsed_config.remove("config") {
        Some(config) => {
            Settings::deserialize(config).map_err(|e| Error::ParseError(e.to_string()))?
        }
        None => Settings::default(),
    };

    settings.ui.propagate();

    // get config as table of top level entries
    let store = Store::build(parsed_config.into_iter()).map_err(Error::StoreError);

    Ok(Config { settings, store })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_file_is_valid() {
        let config = load_txt("").unwrap();
        assert_eq!(config.settings, Settings::default());
    }

    #[test]
    fn partial_settings_uses_default() {
        let toml = r#"[config]
		    ui = {width = 42 }"#;

        let config = load_txt(toml).unwrap();

        assert_eq!(
            config.settings.ui.search.padding,
            Settings::default().ui.search.padding
        );
        assert_ne!(config.settings.ui.width, Settings::default().ui.width);
    }

    #[test]
    fn invalid_entry_keeps_non_default_settings() {
        let toml = r#"a = 1
                    [config.ui]
		    width = 42"#;

        let config = load_txt(toml).unwrap();

        assert_ne!(config.settings, Settings::default());
        assert!(matches!(config.store, Err(Error::StoreError(_))));
    }

    #[test]
    fn extraneous_setting_allowed() {
        let toml = r#"[config]
		    not_a_real_setting = 42"#;

        let config = load_txt(toml).unwrap();

        assert_eq!(config.settings, Settings::default());
    }

    #[test]
    fn nonexistent_path() {
        let result = load_path("nonexistentfile.toml");
        assert!(matches!(result, Err(Error::IoError(_))));
    }

    #[test]
    fn child_settings_override() {
        let toml = r#"[config.ui.search]
		    text_size = 42"#;

        let settings = load_txt(toml).unwrap().settings;

        assert_eq!(settings.ui.common, Default::default());

        assert_eq!(settings.ui.entry, Default::default());

        assert_ne!(settings.ui.search, Default::default());
    }

    #[test]
    fn parent_settings_inherit() {
        let toml = r#"[config.ui]
		    text_size = 42"#;

        let settings = load_txt(toml).unwrap().settings;

        assert_ne!(settings.ui.common, Default::default());

        assert_ne!(settings.ui.entry, Default::default());

        assert_ne!(settings.ui.search, Default::default());
    }

    #[test]
    fn test_one_or_many() {
        use super::one_or_many;
        use serde::de::value::{MapDeserializer, SeqDeserializer, UnitDeserializer};

        let _: Vec<()> = one_or_many(UnitDeserializer::<serde::de::value::Error>::new()).unwrap();

        let seq_de = SeqDeserializer::<_, serde::de::value::Error>::new(std::iter::once(()));
        one_or_many::<Vec<()>, _>(seq_de).unwrap();

        let map_de =
            MapDeserializer::<_, serde::de::value::Error>::new(std::iter::once(("a", "b")));
        one_or_many::<Vec<()>, _>(map_de).unwrap_err();
    }
}
