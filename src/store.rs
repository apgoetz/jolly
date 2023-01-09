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
use serde;
use serde::Deserialize;
use std::error;
use std::fmt;
use std::hash::Hash;
use std::ops::Deref;
use toml;
use urlencoding;

use crate::config::LOGFILE_NAME;
use crate::platform;

// these are the weights for the different kind of matches.
// we prefer each weight to be different so we can differentiate them in the test plan
const FULL_KEYWORD_W: u32 = 100;
const PARTIAL_NAME_W: u32 = 3;
const FULL_NAME_W: u32 = 10;
const PARTIAL_TAG_W: u32 = 2;
const STARTSWITH_TAG_W: u32 = 4;
const FULL_TAG_W: u32 = 6;

#[derive(Debug)]
pub enum Error {
    IOError(std::io::Error),
    ParseError(String),
    BareKeyError(String),
    CustomError(String),
    PlatformError(platform::Error),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::IOError(e) => e.fmt(f),
            Error::PlatformError(e) => e.fmt(f),
            Error::BareKeyError(e) => {
                write!(
                    f,
                    "Invalid {} entry '{}': Jolly entries can only be TOML tables",
                    LOGFILE_NAME, e
                )
            }
            Error::ParseError(e) => {
                write!(f, "TOML Error: ")?;
                e.fmt(f)
            }
            Error::CustomError(s) => f.write_str(s),
        }
    }
}

impl error::Error for Error {}

#[derive(serde::Deserialize, Debug)]
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

#[derive(Debug, Eq, PartialEq, Clone, Hash)]
pub enum EntryType {
    FileEntry(String),
    SystemEntry(String),
}

impl StoreEntry {
    // parse a toml value into a store entry
    fn from_value(name: String, val: toml::Value) -> Result<Self, Error> {
        if !val.is_table() {
            return Err(Error::BareKeyError(name));
        }

        let raw_entry =
            RawStoreEntry::deserialize(val).map_err(|e| Error::ParseError(e.to_string()))?;

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
            name: name.to_string(),
            entry: entry,
            tags: tags,
            keyword: keyword,
        })
    }

    // basic idea: search query consists of multiple filters that
    // are ANDED together. And Each query is run on the name and
    // each tag and ORed together
    //
    // score functions score(item, query)
    //
    // for entry (name='foo', tags = ['abc', '123'])
    //
    // for query = "foo a"
    //
    // overall score = MIN(
    //                   MAX(score('foo', 'foo'), score('abc', 'foo'), score('123', 'foo')),
    //                   MAX(score('foo', 'a'), score('abc', 'a'), score('123', 'a')),
    //                 );
    //
    //
    // search results are a little different for keword entries.
    //
    // for keyword entries, they follow the same normal scoring as
    // seen above, so they show up in results for searchs. But
    // they OR together a special check for (1st search token) == keyword token
    fn score(&self, searchtext: &str) -> u32 {
        // determine if we are doing case sensitive or case - insensitive match
        let change_case = if searchtext == searchtext.to_lowercase() {
            |s: &str| s.to_uppercase()
        } else {
            |s: &str| s.to_string()
        };

        // build temporary strings with the right case
        let name = change_case(&self.name);
        let tags: Vec<_> = self
            .tags
            .iter()
            .map(String::deref)
            .map(change_case)
            .collect();
        let query: Vec<_> = searchtext.split_whitespace().map(change_case).collect();

        // if vec is empty or first element is empty, no score
        if query.len() == 0 || query[0].len() == 0 {
            return 0;
        }

        // check to see if we match a keyword
        let full_keyword = FULL_KEYWORD_W
            * match &self.keyword {
                Keyword::None => false,
                Keyword::RawKeyword(k) => change_case(k) == query[0],
                Keyword::EscapedKeyword(k) => change_case(k) == query[0],
            } as u32;

        let mut running_score = u32::MAX;

        for ref q in query {
            running_score = running_score.min(
                // calculate measures of a match
                [
                    FULL_NAME_W * ((&name == q) as u32),
                    PARTIAL_NAME_W * (name.contains(q) as u32),
                    FULL_TAG_W * (tags.iter().any(|t| t == q) as u32),
                    PARTIAL_TAG_W * (tags.iter().any(|t| t.contains(q)) as u32),
                    STARTSWITH_TAG_W * (tags.iter().any(|t| t.starts_with(q)) as u32),
                ]
                .into_iter()
                .reduce(std::cmp::max)
                .unwrap(),
            );
        }
        running_score.max(full_keyword)
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

    pub fn format_name(&self, searchtext: &str) -> String {
        let param = if let Some((_, back)) = searchtext.split_once(char::is_whitespace) {
            back
        } else {
            "%s"
        };

        self.format(&self.name, param)
    }

    pub fn format_selection(&self, searchtext: &str) -> String {
        let param = if let Some((_, back)) = searchtext.split_once(char::is_whitespace) {
            back
        } else {
            "%s"
        };

        let escaped_param = match self.keyword {
            Keyword::EscapedKeyword(_) => urlencoding::encode(param).into_owned(),
            _ => param.to_string(),
        };

        let s = match &self.entry {
            EntryType::FileEntry(s) => s,
            EntryType::SystemEntry(s) => s,
        };
        self.format(s, escaped_param)
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
    pub fn build<'a, E: Iterator<Item = (String, toml::Value)>>(
        serialized_entries: E,
    ) -> Result<Store, Error> {
        Ok(Store {
            entries: serialized_entries
                .map(|(k, v)| StoreEntry::from_value(k, v))
                .collect::<Result<Vec<_>, _>>()?,
        })
    }

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

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile;

    fn parse_store(text: &str) -> Result<Store, Error> {
        let value: toml::Value = toml::from_str(text).unwrap();

        if let toml::Value::Table(table) = value {
            Store::build(table.into_iter())
        } else {
            panic!("Toml is not a Table")
        }
    }

    #[test]
    fn parse_empty_file() {
        let store = parse_store("").unwrap();
        assert_eq!(store.entries.len(), 0)
    }

    #[test]
    fn case_sensitive() {
        let entry = StoreEntry {
            name: "fOO.txt".to_string(),
            keyword: Keyword::None,
            entry: EntryType::FileEntry("test/location/asdf.txt".to_string()),
            tags: ["FOO"].into_iter().map(str::to_string).collect(),
        };

        // if we give a lowercase query, then default case insensitive match
        assert_eq!(entry.score("fo"), STARTSWITH_TAG_W);
        // if we give a
        assert_eq!(entry.score("FO"), STARTSWITH_TAG_W);
        assert_eq!(entry.score("FOO"), FULL_TAG_W);
        assert_eq!(entry.score("fO"), PARTIAL_NAME_W);
    }

    #[test]
    fn non_keword_score() {
        let entry = StoreEntry {
            name: "foo.txt".to_string(),
            keyword: Keyword::None,
            entry: EntryType::FileEntry("test/location/foo.txt".to_string()),
            tags: ["foo", "bar", "baz"]
                .into_iter()
                .map(str::to_string)
                .collect(),
        };

        assert_eq!(entry.score("tx"), PARTIAL_NAME_W);
        assert_eq!(entry.score("foo"), FULL_TAG_W);
        assert_eq!(entry.score("foo.txt"), FULL_NAME_W);

        assert_eq!(entry.score("ba"), STARTSWITH_TAG_W);
        assert_eq!(entry.score("az"), PARTIAL_TAG_W);

        assert_eq!(entry.score("baz"), FULL_TAG_W);
        assert_eq!(entry.score("bar fo"), STARTSWITH_TAG_W);
        assert_eq!(entry.score("bar az"), PARTIAL_TAG_W);
        assert_eq!(entry.score(""), 0);
    }

    #[test]
    fn keword_score() {
        let entry = StoreEntry {
            name: "foo.txt".to_string(),
            keyword: Keyword::RawKeyword("y".to_string()),
            entry: EntryType::FileEntry("test/location/foo.txt".to_string()),
            tags: ["foo", "bar", "baz"]
                .into_iter()
                .map(str::to_string)
                .collect(),
        };

        // if you dont use a keyword, score normally
        assert_eq!(entry.score("fo"), STARTSWITH_TAG_W);

        // otherwise get big bonus for using keyword
        assert_eq!(entry.score("y foo"), FULL_KEYWORD_W);
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
    fn keyword_search_results() {
        let mut entry = StoreEntry {
            name: "name:%s".to_string(),
            keyword: Keyword::RawKeyword(Default::default()),
            entry: EntryType::FileEntry("file/%s".to_string()),
            tags: Default::default(),
        };

        let raw: Keyword = Keyword::RawKeyword(Default::default());
        let escaped: Keyword = Keyword::EscapedKeyword(Default::default());

        let tests = [
            (&raw, "a b", "name:b", "file/b"),
            (&raw, "a B", "name:B", "file/B"),
            (&raw, "a b c", "name:b c", "file/b c"),
            (&escaped, "a b", "name:b", "file/b"),
            (&escaped, "a b c", "name:b c", "file/b%20c"),
        ];

        for (entry_type, searchtext, formatted_name, formatted_selection) in tests {
            entry.keyword = entry_type.clone();

            assert_eq!(
                formatted_name,
                entry.format_name(searchtext),
                r#"formatted_name:"{}" -> "{}" failed: "#,
                searchtext,
                formatted_name
            );

            assert_eq!(
                formatted_selection,
                entry.format_selection(searchtext),
                r#"format_selection:"{}" -> "{}" failed: "#,
                searchtext,
                formatted_selection
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
        let text = r#"bare_key = 1"#;
        panic!("{:?}", toml::from_str::<toml::Value>(text))
    }

    #[test]
    fn bare_keys_not_allowed() {
        let toml = r#"bare_key = 42"#;
        let text = parse_store(toml);
        assert!(matches!(text, Err(Error::BareKeyError(_))), "{:?}", text)
    }

    #[test]
    fn parse_error() {
        let tests = [
            r#"['asdf']
               location = 1"#,
            r#"['asdf']
               url = 1"#,
            r#"['asdf']
               system = 1"#,
            r#"['asdf']
               keyword = 1"#,
            r#"['asdf']
               escape = 1"#,
            r#"['asdf']
               tags = 1"#,
            r#"['asdf']
               tags = 'foo'"#,
        ];

        for toml in tests {
            assert!(matches!(parse_store(&toml), Err(Error::ParseError(_))));
        }
    }
}
