// contains logic to parse the saved links for jolly
// basic format of jolly storage:
//
// ['filename.txt'] # filename or name for bookmark, can also contain path
// tags = ['foo', 'text', 'baz']
// location = '/optional/path/to/filename'
// url = 'http://example.com' #can contain mozilla style query string (single %s). Defaults to escape = true
// system = 'cmd to run'# can contain mozilla style query string (single %s)
// keyword = 'k' # keyword used for mozilla style query strings
// escape = true # only valid for keyword entries, determines if query string is escaped.
use serde::Deserialize;
use std::error;
use std::fmt;
use std::fs;
use std::hash::Hash;
use std::path::Path;
use toml;
use urlencoding;

use crate::platform;

const FULL_KEYWORD_W: u32 = 100;
const PARTIAL_NAME_W: u32 = 2;
const FULL_NAME_W: u32 = 4;
const PARTIAL_TAG_W: u32 = 1;
const FULL_TAG_W: u32 = 2;

#[derive(Debug)]
pub enum Error {
    IOError(std::io::Error),
    ParseError(toml::de::Error),
    CustomError(String),
    PlatformError(platform::Error),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::IOError(e) => e.fmt(f),
            Error::PlatformError(e) => e.fmt(f),
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
    url: Option<String>,
    system: Option<String>,
    keyword: Option<String>,
    escape: Option<bool>,
    tags: Option<Vec<String>>,
}

#[derive(Debug, Eq, PartialEq, Clone, Hash)]
enum Keyword {
    None,
    RawKeyword(String),
    EscapedKeyword(String),
}

#[derive(Debug, Eq, PartialEq, Clone, Hash)]
pub struct StoreEntry {
    name: String,
    entry: EntryType,
    tags: Vec<String>,
    keyword: Keyword,
}

struct SearchResult<'a> {
    entry: &'a StoreEntry,
    query: &'a str,
    param: &'a str,
}

impl<'a> SearchResult<'a> {
    fn new(entry: &'a StoreEntry, searchtext: &'a str) -> Self {
        let (query, param) = if let Some((front, back)) = searchtext.split_once(char::is_whitespace)
        {
            (front, back)
        } else {
            (searchtext, "%s")
        };
        Self {
            entry,
            query,
            param,
        }
    }

    fn escaped_param(&self) -> String {
        match self.entry.keyword {
            Keyword::EscapedKeyword(_) => urlencoding::encode(self.param).into_owned(),
            _ => self.param.to_string(),
        }
    }
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

        let keyword = if let Some(keyword) = raw_entry.keyword {
            if raw_entry.url.is_some() || raw_entry.escape.unwrap_or(false) {
                Keyword::EscapedKeyword(keyword)
            } else {
                Keyword::RawKeyword(keyword)
            }
        } else {
            Keyword::None
        };

        let is_system = raw_entry.system.is_some();

        let location = match (raw_entry.location, raw_entry.url, raw_entry.system) {
            (Some(loc), None, None) => loc,
            (None, Some(loc), None) => loc,
            (None, None, Some(loc)) => loc,
            (None, None, None) => name.to_string(),
            _ => {
                return Err(Error::CustomError(format!(
                    "Error with {}: Only allow one of location/url/system",
                    &name
                )))
            }
        };

        let entry = if is_system {
            EntryType::SystemEntry(location)
        } else {
            EntryType::FileEntry(location)
        };

        let tags = match raw_entry.tags {
            Some(tags) => tags,
            None => Vec::new(),
        };

        Ok(StoreEntry {
            name: name,
            entry: entry,
            tags: tags,
            keyword: keyword,
        })
    }

    // score an entry based on information
    fn score(&self, searchtext: &str) -> u32 {
        let query = SearchResult::new(self, searchtext).query;

        if query.len() == 0 {
            return 0;
        }

        let full_keyword = match &self.keyword {
            Keyword::None => false,
            Keyword::RawKeyword(k) => k == query,
            Keyword::EscapedKeyword(k) => k == query,
        };

        // calculate measures of a match
        let full_name = self.name == query;
        let partial_name = !full_name && self.name.contains(query);

        let full_tag = self.tags.iter().any(|t| t == query);
        let partial_tag = !full_tag && self.tags.iter().any(|t| t.contains(query));

        // calculate "score" as crossproduct of weights and values
        let vals = [full_keyword, partial_name, full_name, partial_tag, full_tag].map(u32::from);
        let weights = [
            FULL_KEYWORD_W,
            PARTIAL_NAME_W,
            FULL_NAME_W,
            PARTIAL_TAG_W,
            FULL_TAG_W,
        ];
        vals.iter()
            .zip(weights)
            .map(|(&a, b)| a * b)
            .reduce(|a, b| a + b)
            .unwrap()
    }

    // format example:
    //

    fn format<S: AsRef<str>>(&self, fmt_str: &str, searchtext: S) -> String {
        if self.keyword == Keyword::None {
            return fmt_str.to_string();
        }

        fmt_str
            .split("%%")
            .map(|s| s.replace("%s", searchtext.as_ref()))
            .collect::<Vec<_>>()
            .join("%")
    }

    pub fn formatted_name(&self, searchtext: &str) -> String {
        self.format(&self.name, SearchResult::new(self, searchtext).param)
    }

    pub fn format_selection(&self, searchtext: &str) -> String {
        let param = SearchResult::new(self, searchtext).escaped_param();
        let s = match &self.entry {
            EntryType::FileEntry(s) => s,
            EntryType::SystemEntry(s) => s,
        };
        self.format(s, param)
    }

    pub fn handle_selection(&self, searchtext: &str) -> Result<(), Error> {
        let func = match &self.entry {
            EntryType::FileEntry(_) => platform::open_file,
            EntryType::SystemEntry(_) => platform::system,
        };
        func(&self.format_selection(searchtext)).map_err(Error::PlatformError)
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
            keyword: Keyword::None,
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
                    keyword: Keyword::None,
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
                    keyword: Keyword::None,
                    entry: EntryType::FileEntry("test/location".to_string()),
                    tags: [].into_iter().map(str::to_string).collect(),
                },
            ),
            (
                r#"['test/location/foo.txt']
		    tags = ["foo", 'bar', 'baz']"#,
                StoreEntry {
                    name: "test/location/foo.txt".to_string(),
                    keyword: Keyword::None,
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
            keyword: Keyword::None,
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
            keyword: Keyword::None,
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

    #[test]
    fn search_results() {
        let mut entry = StoreEntry {
            name: Default::default(),
            keyword: Keyword::RawKeyword(Default::default()),
            entry: EntryType::FileEntry(Default::default()),
            tags: Default::default(),
        };

        let raw: Keyword = Keyword::RawKeyword(Default::default());
        let escaped: Keyword = Keyword::EscapedKeyword(Default::default());

        let tests = [
            (&raw, "a b", "a", "b", "b"),
            (&raw, "a b c", "a", "b c", "b c"),
            (&escaped, "a b", "a", "b", "b"),
            (&escaped, "a b c", "a", "b c", "b%20c"),
        ];

        for (entry_type, searchtext, query, rawparam, escapedparam) in tests {
            entry.keyword = entry_type.clone();
            let res = SearchResult::new(&entry, searchtext);

            assert_eq!(
                query, res.query,
                r#"query:"{}" -> "{}" failed: "#,
                searchtext, query
            );

            assert_eq!(
                rawparam, res.param,
                r#"param:"{}" -> "{}" failed: "#,
                searchtext, rawparam
            );

            assert_eq!(
                escapedparam,
                res.escaped_param(),
                r#"escaped_param:"{}" -> "{}" failed: "#,
                searchtext,
                escapedparam
            );
        }
    }

    #[test]
    fn test_format() {
        let entry = StoreEntry {
            name: Default::default(),
            keyword: Keyword::RawKeyword(Default::default()),
            entry: EntryType::FileEntry(Default::default()),
            tags: Default::default(),
        };

        let tests = [
            ("%s", "a", "a"),
            ("test %s", "a", "test a"),
            ("%%s", "a", "%s"),
            ("%%%s", "a", "%a"),
            ("%s", "a a", "a a"),
        ];

        for test in tests {
            assert_eq!(
                test.2,
                entry.format(test.0, test.1),
                r#"format("{}", "{}") -> "{}" failed: "#,
                test.0,
                test.1,
                test.2
            );
        }
    }

    // this test is just a playground to see what toml is rendered as
    #[test]
    #[ignore]
    fn toml_test() {
        let text = r#"[foo.txt]"#;
        panic!("{:?}", toml::from_str::<toml::Value>(text))
    }
}
