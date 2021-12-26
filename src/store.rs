// contains logic to parse the saved links for jolly
// basic format of jolly storage:
//
// ['filename.txt'] # filename or name for bookmark, can also contain path
// tags = ['foo', 'text', 'baz']
// location = '/optional/path/to/filename'
// url = 'http://example.com' #can contain mozilla style query string (single %s)
// system = 'cmd to run'# can contain mozilla style query string (single %s)
// keyword = 'k' # keyword used for mozilla style query strings
use serde::Deserialize;
use std::error;
use std::fmt;
use std::fs;
use std::hash::Hash;
use std::path::Path;
use toml;

const PARTIAL_NAME_W: u32 = 2;
const FULL_NAME_W: u32 = 4;
const PARTIAL_TAG_W: u32 = 1;
const FULL_TAG_W: u32 = 2;

#[derive(Debug)]
pub enum Error {
    IOError(std::io::Error),
    ParseError(toml::de::Error),
    CustomError(String),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::IOError(e) => e.fmt(f),
            Error::ParseError(e) => {
                write!(f, "TOML Error: ")?;
                e.fmt(f)
            }
            Error::CustomError(s) => f.write_str(s),
        }
    }
}

impl error::Error for Error {}

#[derive(Deserialize, Debug)]
struct RawStoreEntry {
    location: Option<String>,
    system: Option<String>,
    tags: Option<Vec<String>>,
}

#[derive(Debug, Eq, PartialEq, Clone, Hash)]
pub struct StoreEntry {
    pub name: String,
    pub entry: EntryType,
    pub tags: Vec<String>,
}

#[derive(Debug, Eq, PartialEq, Clone, Hash)]
pub enum EntryType {
    FileEntry(String),
    SystemEntry(String),
}

impl StoreEntry {
    // parse a toml value into a store entry
    fn from_value(name: String, val: toml::Value) -> Result<Self, Error> {
        let raw_entry: RawStoreEntry = val.try_into().map_err(|e| Error::ParseError(e))?;

        let location = match &raw_entry.location {
            Some(loc) => loc,
            None => &name,
        }
        .to_string();
        let entry;

        if let Some(s) = raw_entry.system {
            entry = EntryType::SystemEntry(s);
        } else {
            entry = EntryType::FileEntry(location);
        }

        let tags = match raw_entry.tags {
            Some(tags) => tags,
            None => Vec::new(),
        };

        Ok(StoreEntry {
            name: name.to_string(),
            entry: entry,
            tags: tags,
        })
    }

    // score an entry based on information
    fn score(&self, query: &str) -> u32 {
        if query.len() == 0 {
            return 0;
        }

        // calculate measures of a match
        let full_name = self.name == query;
        let partial_name = !full_name && self.name.contains(query);

        let full_tag = self.tags.iter().any(|t| t == query);
        let partial_tag = !full_tag && self.tags.iter().any(|t| t.contains(query));

        // calculate "score" as crossproduct of weights and values
        let vals = [partial_name, full_name, partial_tag, full_tag].map(u32::from);
        let weights = [PARTIAL_NAME_W, FULL_NAME_W, PARTIAL_TAG_W, FULL_TAG_W];
        vals.iter()
            .zip(weights)
            .map(|(&a, b)| a * b)
            .reduce(|a, b| a + b)
            .unwrap()
    }
}

#[derive(Debug, Default, Clone)]
pub struct Store {
    entries: Vec<StoreEntry>,
}

impl Store {
    pub fn find_matches(&self, query: &str) -> Vec<&StoreEntry> {
        // get indicies of all entries with scores greater than zero
        let mut matches: Vec<_> = self
            .entries
            .iter()
            .map(|entry| entry.score(query))
            .enumerate()
            .filter(|s| s.1 > 0)
            .rev() // flip order: now we prefer LAST entries in file
            .collect::<Vec<_>>();

        // sort by score
        matches.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());

        // get references to entries in sorted order
        matches.iter().map(|s| &self.entries[s.0]).collect()
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }
}

pub fn load_store<P: AsRef<Path>>(path: P) -> Result<Store, Error> {
    match fs::read_to_string(path) {
        Ok(txt) => parse_store(&txt),
        Err(err) => Err(Error::IOError(err)),
    }
}

fn parse_store(text: &str) -> Result<Store, Error> {
    let value: toml::Value = toml::from_str(text).map_err(|e| Error::ParseError(e))?;
    let table = match value {
        toml::Value::Table(t) => t,
        _ => return Err(Error::CustomError("Toml is not a Table".to_string())),
    };

    Ok(Store {
        entries: table
            .into_iter()
            .map(|(f, v)| StoreEntry::from_value(f, v))
            .collect::<Result<Vec<_>, _>>()?,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile;

    #[test]
    fn parse_empty_file() {
        let store = parse_store("").unwrap();
        assert_eq!(store.entries.len(), 0)
    }

    #[test]
    fn entry_score() {
        let entry = StoreEntry {
            name: "foo.txt".to_string(),
            entry: EntryType::FileEntry("test/location/foo.txt".to_string()),
            tags: ["foo", "bar", "baz"]
                .into_iter()
                .map(str::to_string)
                .collect(),
        };

        assert_eq!(entry.score("fo"), PARTIAL_NAME_W + PARTIAL_TAG_W);
        assert_eq!(entry.score("foo"), PARTIAL_NAME_W + FULL_TAG_W);
        assert_eq!(entry.score("foo.txt"), FULL_NAME_W);

        assert_eq!(entry.score("ba"), PARTIAL_TAG_W);
        assert_eq!(entry.score("baz"), FULL_TAG_W);
        assert_eq!(entry.score(""), 0);
    }

    #[test]
    fn find_entries() {
        let toml = r#"['foo']
		      tags = ["foo", 'bar', 'quu']
                      location = "test/location"

                      ['asdf']
                      tags = ["bar", "quux"]
                      location = "test/location""#;

        let store = parse_store(toml).unwrap();

        let tests = [
            ("fo", vec!["foo"]),
            ("foo", vec!["foo"]),
            ("bar", vec!["asdf", "foo"]), // all things being equal, prefer "newer" entries
            ("asd", vec!["asdf"]),
            ("asdf", vec!["asdf"]),
            ("quu", vec!["foo", "asdf"]), // since quu is a full match for foo entry, it ranks higher
            ("quux", vec!["asdf"]),
            ("", vec![]),
        ];

        for (query, results) in tests {
            let matches = store.find_matches(query);
            assert_eq!(
                results.len(),
                matches.len(),
                "test: {} -> {:?}",
                query,
                results
            );

            let r_entries = results
                .into_iter()
                .map(|e| store.entries.iter().find(|e2| e2.name == e).unwrap());

            for (l, r) in matches.into_iter().zip(r_entries) {
                assert_eq!(
                    l,
                    r,
                    "lscore: {} rscore: {}",
                    l.score(query),
                    r.score(query)
                );
            }
        }
    }

    #[test]
    fn parse_single_file_entry() {
        let pairs = [
            (
                r#"['foo.txt']
		    tags = ["foo", 'bar', 'baz']
                    location = "test/location""#,
                StoreEntry {
                    name: "foo.txt".to_string(),
                    entry: EntryType::FileEntry("test/location".to_string()),
                    tags: ["foo", "bar", "baz"]
                        .into_iter()
                        .map(str::to_string)
                        .collect(),
                },
            ),
            (
                r#"['foo.txt']
                    location = "test/location""#,
                StoreEntry {
                    name: "foo.txt".to_string(),
                    entry: EntryType::FileEntry("test/location".to_string()),
                    tags: [].into_iter().map(str::to_string).collect(),
                },
            ),
            (
                r#"['test/location/foo.txt']
		    tags = ["foo", 'bar', 'baz']"#,
                StoreEntry {
                    name: "test/location/foo.txt".to_string(),
                    entry: EntryType::FileEntry("test/location/foo.txt".to_string()),
                    tags: ["foo", "bar", "baz"]
                        .into_iter()
                        .map(str::to_string)
                        .collect(),
                },
            ),
        ];

        for (toml, expected_entry) in pairs {
            let store = parse_store(toml).unwrap();
            let entry = &store.entries[0];

            assert!(store.entries.len() == 1);
            assert_eq!(&expected_entry, entry);
        }
    }

    #[test]
    fn parse_error() {
        let toml = r#"['asdf']
		    tags = ["foo", 'bar', 'baz'"#;

        assert!(matches!(parse_store(&toml), Err(Error::ParseError(_))));
    }

    #[test]
    fn system_entry() {
        let dir = tempfile::tempdir().unwrap();
        let dirname = dir.path().to_string_lossy();
        let toml = format!(
            r#"['{}']
                    system = 'foo bar'
		    tags = ["foo", 'bar', 'baz']"#,
            dirname
        );
        let expected_entry = StoreEntry {
            name: dirname.to_string(),
            entry: EntryType::SystemEntry("foo bar".to_string()),
            tags: ["foo", "bar", "baz"]
                .into_iter()
                .map(str::to_string)
                .collect(),
        };

        let store = parse_store(&toml).unwrap();
        let entry = &store.entries[0];

        assert!(store.entries.len() == 1);
        assert_eq!(&expected_entry, entry);
    }

    #[test]
    fn single_dir_entry() {
        let dir = tempfile::tempdir().unwrap();
        let dirname = dir.path().to_string_lossy();
        let toml = format!(
            r#"['{}']
		    tags = ["foo", 'bar', 'baz']"#,
            dirname
        );
        let expected_entry = StoreEntry {
            name: dirname.to_string(),
            entry: EntryType::FileEntry(dirname.to_string()),
            tags: ["foo", "bar", "baz"]
                .into_iter()
                .map(str::to_string)
                .collect(),
        };

        let store = parse_store(&toml).unwrap();
        let entry = &store.entries[0];

        assert!(store.entries.len() == 1);
        assert_eq!(&expected_entry, entry);
    }

    #[test]
    fn nonexistent_path() {
        let result = load_store("nonexistentfile.toml");
        assert!(matches!(result, Err(Error::IOError(_))));
    }

    // this test is just a playground to see what toml is rendered as
    #[test]
    #[ignore]
    fn toml_test() {
        let text = r#"[foo.txt]"#;
        panic!("{:?}", toml::from_str::<toml::Value>(text))
    }
}
