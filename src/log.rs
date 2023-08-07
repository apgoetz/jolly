use env_logger::Builder;
use serde::Deserialize;

use crate::config::one_or_many;
use crate::error;

#[derive(Deserialize, Debug, Clone, PartialEq, Default)]
#[serde(default)]
pub struct LogSettings {
    file: Option<String>,
    #[serde(deserialize_with = "one_or_many")]
    filters: Vec<String>,
}

impl LogSettings {
    pub fn init_logger(&self) -> Result<(), error::Error> {
        self.build_logger().map(|b| {
            if let Some(mut b) = b {
                b.init()
            }
        })
    }

    fn build_logger(&self) -> Result<Option<Builder>, error::Error> {
        use env_logger::fmt::Target;

        let mut builder = Builder::new();
        builder
            .parse_filters(&self.filters.join(","))
            .format_timestamp_micros();

        if let Some(filename) = &self.file {
            if filename == "stdout" {
                builder.target(Target::Stdout);
            } else if filename == "stderr" {
                builder.target(Target::Stderr);
            } else {
                let f = std::fs::OpenOptions::new()
                    .append(true)
                    .create(true)
                    .open(filename)
                    .map_err(|e| error::Error::IoError(self.file.clone(), e))?;
                builder.target(env_logger::fmt::Target::Pipe(Box::new(f)));
            }
            Ok(Some(builder))
        } else {
            Ok(None)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::LogSettings;
    use crate::error;
    use ::log;
    use env_logger::Builder;
    use log::Log;
    use tempfile;

    fn file_logger<F: AsRef<std::path::Path>>(f: F) -> Result<Builder, error::Error> {
        LogSettings {
            file: Some(f.as_ref().to_string_lossy().to_string()),
            filters: vec!["trace".into()],
        }
        .build_logger()
        .map(Option::unwrap)
    }

    #[test]
    fn test_log_appends() {
        let dir = tempfile::tempdir().unwrap();

        let filename = dir.path().join("a");

        let record = log::RecordBuilder::new().build();

        for i in 1..3 {
            let logger = file_logger(&filename).unwrap().build();
            logger.log(&record);

            std::mem::drop(logger);

            let linecount = std::fs::read_to_string(&filename).unwrap().lines().count();

            assert_eq!(linecount, i);
        }
    }
}
