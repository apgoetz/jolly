// contains logic for parsing jolly config file.
// a config file consists of settings and a store
// settings are parameters for the program
// store represents the links that are stored in jolly

use crate::{error, settings::Settings, store};
use serde::Deserialize;
use std::{fs, path};
use toml;

pub const LOGFILE_NAME: &str = "jolly.toml";

pub fn load_config() -> Result<(Settings, store::Store), String> {
    let logfile = get_logfile().map_err(|e| e.to_string())?;
    load_path(logfile).map_err(|e| e.to_string())
}

fn load_path<P: AsRef<path::Path>>(path: P) -> Result<(Settings, store::Store), error::Error> {
    let txt = fs::read_to_string(path).map_err(error::Error::IoError)?;
    load_txt(&txt)
}

fn load_txt(txt: &str) -> Result<(Settings, store::Store), error::Error> {
    let value: toml::Value =
        toml::from_str(txt).map_err(|e| error::Error::ParseError(e.to_string()))?;

    let mut parsed_config = match value {
        toml::Value::Table(t) => t,
        _ => return Err(error::Error::ParseError("entry is not a Table".to_string())),
    };

    // if we have a settings entry use it, otherwise deserialize something empty and rely on serde defaults
    let config = match parsed_config.remove("config") {
        Some(config) => config,
        None => toml::Value::Table(toml::value::Table::new()),
    };

    let settings =
        Settings::deserialize(config).map_err(|e| error::Error::ParseError(e.to_string()))?;

    // get config as table of top level entries
    let store = store::Store::build(parsed_config.into_iter()).map_err(error::Error::StoreError)?; // todo fix unwrap

    Ok((settings, store))
}

fn get_logfile() -> Result<path::PathBuf, error::Error> {
    let local_path = path::Path::new(LOGFILE_NAME);
    if local_path.exists() {
        return Ok(local_path.to_path_buf());
    }

    let config_dir = dirs::config_dir().ok_or(error::Error::CustomError(
        "Cannot Determine Config Dir".to_string(),
    ))?;
    let config_path = config_dir.join(LOGFILE_NAME);
    if config_path.exists() {
        Ok(config_path)
    } else {
        Err(error::Error::CustomError(format!(
            "Cannot find {}",
            LOGFILE_NAME
        )))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_file_is_valid() {
        let (settings, _store) = load_txt("").unwrap();
        assert_eq!(settings, Settings::default());
    }

    #[test]
    fn partial_settings_uses_default() {
        let toml = r#"['config']
		    ui_width = 42"#;

        let (settings, _store) = load_txt(toml).unwrap();

        assert_eq!(settings.ui_max_results, Settings::default().ui_max_results);
        assert_ne!(settings.ui_width, Settings::default().ui_width);
    }

    #[test]
    fn extraneous_setting_allowed() {
        let toml = r#"['config']
		    not_a_real_setting = 42"#;

        let (settings, _store) = load_txt(toml).unwrap();

        assert_eq!(settings, Settings::default());
    }

    #[test]
    fn nonexistent_path() {
        let result = load_path("nonexistentfile.toml");
        assert!(matches!(result, Err(error::Error::IoError(_))));
    }
}
