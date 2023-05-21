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

use toml;

use crate::entry;

#[derive(Debug, Default, Clone)]
pub struct Store {
    entries: Vec<entry::StoreEntry>,
}

impl Store {
    pub fn build<'a, E: Iterator<Item = (String, toml::Value)>>(
        serialized_entries: E,
    ) -> Result<Store, entry::Error> {
        Ok(Store {
            entries: serialized_entries
                .map(|(k, v)| entry::StoreEntry::from_value(k, v))
                .collect::<Result<Vec<_>, _>>()?,
        })
    }

    pub fn find_matches(&self, query: &str) -> Vec<&entry::StoreEntry> {
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
pub mod tests {
    use super::*;

    pub fn parse_store(text: &str) -> Result<Store, entry::Error> {
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

            let r_entries = results.into_iter().map(|e| {
                store
                    .entries
                    .iter()
                    .find(|e2| e2.format_name(query) == e)
                    .unwrap()
            });

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
    fn bare_keys_not_allowed() {
        let toml = r#"bare_key = 42"#;
        let text = parse_store(toml);
        assert!(
            matches!(text, Err(entry::Error::BareKeyError(_))),
            "{:?}",
            text
        )
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
            assert!(matches!(
                parse_store(&toml),
                Err(entry::Error::ParseError(_))
            ));
        }
    }
}
