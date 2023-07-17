// contains logic for displaying entries

use std::error;
use std::fmt;
use std::ops::Deref;

use serde::Deserialize;
use url::Url;

use crate::icon::Icon;
use crate::theme;
use crate::ui;
use crate::{icon, platform};

use crate::config::LOGFILE_NAME;

// these are the weights for the different kind of matches.
// we prefer each weight to be different so we can differentiate them in the test plan
const FULL_KEYWORD_W: u32 = 100;
const PARTIAL_NAME_W: u32 = 3;
const FULL_NAME_W: u32 = 10;
const PARTIAL_TAG_W: u32 = 2;
const STARTSWITH_TAG_W: u32 = 4;
const FULL_TAG_W: u32 = 6;

pub type EntryId = usize;

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

// theme settings for each shown entry result
#[derive(serde::Deserialize, Debug, Clone, PartialEq)]
#[serde(default)]
pub struct EntrySettings {
    #[serde(flatten)]
    common: ui::InheritedSettings,
    description_size: u16,
}

impl EntrySettings {
    pub fn propagate(&mut self, parent: &ui::InheritedSettings) {
        self.common.propagate(parent);
    }
}

impl Default for EntrySettings {
    fn default() -> Self {
        let inherited = ui::InheritedSettings::default();
        let description_size = (inherited.text_size() as f32 * 0.8).round() as u16;
        Self {
            common: inherited,
            description_size: description_size,
        }
    }
}

#[derive(serde::Deserialize, Debug)]
struct RawStoreEntry {
    location: Option<String>,
    url: Option<String>,
    system: Option<String>,
    keyword: Option<String>,
    escape: Option<bool>,
    #[serde(alias = "desc")]
    description: Option<String>,
    tags: Option<Vec<String>>,
    icon: Option<String>,
}

#[derive(Debug, Eq, PartialEq, Clone, Hash)]
enum Keyword {
    None,
    RawKeyword(String),
    EscapedKeyword(String),
}

#[derive(Debug, Clone, Hash)]
pub struct StoreEntry {
    name: String,
    description: Option<String>,
    entry: EntryType,
    tags: Vec<String>,
    keyword: Keyword,
    icon_type: icon::IconType,
    icon: Option<Icon>,
}

#[derive(Debug, Eq, PartialEq, Clone, Hash)]
pub enum EntryType {
    FileEntry(String),
    SystemEntry(String),
}

impl StoreEntry {
    // parse a toml value into a store entry
    pub fn from_value(name: String, val: toml::Value) -> Result<Self, Error> {
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

        let icon_type = if let Some(p) = raw_entry.icon {
            icon::IconType::custom(p)
        } else {
            match &entry {
                EntryType::SystemEntry(loc) => icon::IconType::file(loc),
                EntryType::FileEntry(loc) => {
                    let parsed_loc = format_param(loc, "");

                    if let Ok(url) = Url::parse(&parsed_loc) {
                        icon::IconType::url(url)
                    } else {
                        icon::IconType::file(parsed_loc)
                    }
                }
            }
        };

        Ok(StoreEntry {
            name: name.to_string(),
            description: raw_entry.description,
            entry: entry,
            tags: tags,
            keyword: keyword,
            icon_type,
            icon: None,
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
    pub fn score(&self, searchtext: &str) -> u32 {
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

    pub fn format_name(&self, searchtext: &str) -> String {
        if self.keyword == Keyword::None {
            return self.name.clone();
        }

        let param = if let Some((_, back)) = searchtext.split_once(char::is_whitespace) {
            back
        } else {
            "%s"
        };

        format_param(&self.name, param)
    }

    pub fn format_selection(&self, searchtext: &str) -> String {
        let param = if let Some((_, back)) = searchtext.split_once(char::is_whitespace) {
            back
        } else {
            "%s"
        };

        let s = match &self.entry {
            EntryType::FileEntry(s) => s,
            EntryType::SystemEntry(s) => s,
        };

        let escaped_param = match self.keyword {
            Keyword::EscapedKeyword(_) => urlencoding::encode(param).into_owned(),
            Keyword::None => return s.clone(),
            _ => param.to_string(),
        };

        format_param(s, escaped_param)
    }

    pub fn handle_selection(&self, searchtext: &str) -> Result<(), Error> {
        let func = match &self.entry {
            EntryType::FileEntry(_) => platform::open_file,
            EntryType::SystemEntry(_) => platform::system,
        };
        let selection = self.format_selection(searchtext);
        func(&selection).map_err(Error::PlatformError)
    }

    pub fn build_entry<'a, F, Message, Renderer>(
        &'a self,
        message_func: F,
        searchtext: &str,
        settings: &ui::UISettings,
        selected: bool,
        my_id: EntryId,
    ) -> iced_native::Element<'a, Message, Renderer>
    where
        F: 'static + Copy + Fn(EntryId) -> Message,
        Message: 'static + Clone,
        Renderer: iced_native::renderer::Renderer<Theme = theme::Theme> + 'a,
        Renderer: iced_native::text::Renderer,
        Renderer: iced_native::image::Renderer<Handle = iced::widget::image::Handle>,
    {
        let text_color = if selected {
            settings.theme.selected_text_color.clone()
        } else {
            settings.theme.text_color.clone()
        };

        let button_style = if selected {
            theme::ButtonStyle::Selected
        } else {
            theme::ButtonStyle::Transparent
        };

        let text_color: iced_native::Color = text_color.into();

        let title_text = iced::widget::text::Text::new(self.format_name(searchtext))
            .size(settings.entry.common.text_size())
            .style(iced_native::Color::from(text_color))
            .horizontal_alignment(iced_native::alignment::Horizontal::Left)
            .vertical_alignment(iced_native::alignment::Vertical::Center);

        let description = match &self.description {
            Some(desc) => {
                let paragraphs = desc_to_paragraphs(desc);
                let paragraphs = paragraphs
                    .unwrap_or(vec![desc.to_string()])
                    .into_iter()
                    .map(|paragraph| {
                        iced::widget::text::Text::new(paragraph)
                            .size(settings.entry.description_size)
                            .style(iced_native::Color::from(text_color))
                            .horizontal_alignment(iced_native::alignment::Horizontal::Left)
                            .vertical_alignment(iced_native::alignment::Vertical::Center)
                            .into()
                    })
                    .collect();
                iced::widget::Column::with_children(paragraphs).width(iced_native::Length::Fill)
            }
            None => iced::widget::Column::new(),
        };

        let icon = iced::widget::image::Image::new(
            self.icon
                .clone()
                .unwrap_or_else(|| icon::default_icon(&settings.icon)),
        );

        let icon = icon
            .height(settings.entry.common.text_size())
            .width(settings.entry.common.text_size());

        let icon_row = iced::widget::Row::new()
            .height(iced::Length::Fixed(
                (settings.entry.common.text_size() + 4) as f32,
            ))
            .spacing(2)
            .align_items(iced_native::Alignment::Center)
            .push(icon)
            .push(title_text);

        let column = iced::widget::Column::new()
            .width(iced_native::Length::Fill)
            .push(icon_row)
            .push(description);

        // need an empty container to create padding around title.
        // let _container =
        //     iced::widget::container::Container::new(title_text).padding::<u16>(0u16.into());

        let button = iced::widget::button::Button::new(column)
            .on_press(message_func(my_id))
            .style(button_style)
            .width(iced_native::Length::Fill);

        let element: iced_native::Element<'_, _, _> = button.into();
        element
    }

    // pull out the icon type of this entry in preparation for
    // determing it. current icontype is replaced with pending value
    pub fn icontype(&self) -> &icon::IconType {
        &self.icon_type
    }

    pub fn icon(&mut self, icon: Icon) {
        self.icon = Some(icon);
    }

    pub fn icon_loaded(&self) -> bool {
        self.icon.is_some()
    }
}

fn format_param<S: AsRef<str>>(fmt_str: &str, searchtext: S) -> String {
    fmt_str
        .split("%%")
        .map(|s| s.replace("%s", searchtext.as_ref()))
        .collect::<Vec<_>>()
        .join("%")
}

fn desc_to_paragraphs(desc: &str) -> Option<Vec<String>> {
    use pulldown_cmark::{CodeBlockKind, Event, Parser, Tag};
    let p = Parser::new(desc);
    let mut last_tag = None;
    let mut result = Vec::new();
    let mut cur_paragraph = String::new();
    for event in p {
        match event {
            Event::Start(tag) => {
                // do not allow nested elements. Only list of paragraphs or indented code blocks
                if last_tag.is_some() {
                    return None;
                }

                if tag == Tag::Paragraph || tag == Tag::CodeBlock(CodeBlockKind::Indented) {
                    last_tag = Some(tag);
                    cur_paragraph = String::new();
                } else {
                    // we saw any other kind of tag. Not allowed, abort.
                    return None;
                }
            }
            Event::End(tag) => {
                if Some(tag) != last_tag {
                    return None;
                } else {
                    last_tag = None;
                    result.push(cur_paragraph.clone());
                }
            }
            Event::Text(txt) => cur_paragraph.push_str(&txt),
            Event::Code(txt) => cur_paragraph.push_str(&txt),
            Event::SoftBreak => cur_paragraph.push_str(" "),
            _ => return None,
        }
    }
    Some(result)
}

#[cfg(test)]
mod tests {
    use crate::icon::IconType;

    use super::*;
    use tempfile;

    // lets cheat and use the hash of an entry for partial equivalence
    // good enough for testing
    impl std::cmp::PartialEq for StoreEntry {
        fn eq(&self, other: &Self) -> bool {
            use std::collections::hash_map::DefaultHasher;
            use std::hash::{Hash, Hasher};
            let mut sh = DefaultHasher::new();
            let mut oh = DefaultHasher::new();
            self.hash(&mut sh);
            other.hash(&mut oh);
            sh.finish() == oh.finish()
        }
    }

    fn parse_entry(text: &str) -> StoreEntry {
        let value: toml::Value = toml::from_str(text).unwrap();

        if let toml::Value::Table(table) = value {
            let (k, v) = table.into_iter().next().unwrap();
            StoreEntry::from_value(k, v).unwrap()
        } else {
            panic!("Toml is not a Table")
        }
    }

    #[test]
    fn case_sensitive() {
        let entry = parse_entry(
            r#"['fOO.txt']
                location = "test/location/asdf.txt"
		tags = ['FOO']"#,
        );

        // if we give a lowercase query, then default case insensitive match
        assert_eq!(entry.score("fo"), STARTSWITH_TAG_W);
        // if we give a
        assert_eq!(entry.score("FO"), STARTSWITH_TAG_W);
        assert_eq!(entry.score("FOO"), FULL_TAG_W);
        assert_eq!(entry.score("fO"), PARTIAL_NAME_W);
    }

    #[test]
    fn non_keword_score() {
        let entry = parse_entry(
            r#"['foo.txt']
                location = "test/location/foo.txt"
		tags = ["foo", "bar", "baz"]"#,
        );

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
        let entry = parse_entry(
            r#"['foo.txt']
                location = "test/location/foo.txt"
                keyword = "y"
		tags = ["foo", "bar", "baz"]"#,
        );

        // if you dont use a keyword, score normally
        assert_eq!(entry.score("fo"), STARTSWITH_TAG_W);

        // otherwise get big bonus for using keyword
        assert_eq!(entry.score("y foo"), FULL_KEYWORD_W);
    }

    #[test]
    fn parse_single_file_entry() {
        let pairs = [
            (
                r#"['foo.txt']
		    tags = ["foo", 'bar', 'baz']
                    description = "asdf"
                    location = "test/location""#,
                StoreEntry {
                    name: "foo.txt".to_string(),
                    description: Some("asdf".to_string()),
                    keyword: Keyword::None,
                    entry: EntryType::FileEntry("test/location".to_string()),
                    tags: ["foo", "bar", "baz"]
                        .into_iter()
                        .map(str::to_string)
                        .collect(),
                    icon: None,
                    icon_type: IconType::file("test/location"),
                },
            ),
            (
                r#"['foo.txt']
                    location = "test/location""#,
                StoreEntry {
                    name: "foo.txt".to_string(),
                    description: None,
                    keyword: Keyword::None,
                    entry: EntryType::FileEntry("test/location".to_string()),
                    tags: [].into_iter().map(str::to_string).collect(),
                    icon: None,
                    icon_type: IconType::file("test/location"),
                },
            ),
            (
                r#"['foo.txt']
                    location = "tel:12345""#,
                StoreEntry {
                    name: "foo.txt".to_string(),
                    description: None,
                    keyword: Keyword::None,
                    entry: EntryType::FileEntry("tel:12345".to_string()),
                    tags: [].into_iter().map(str::to_string).collect(),
                    icon: None,
                    icon_type: IconType::url(url::Url::parse("tel:12345").unwrap()),
                },
            ),
            (
                r#"['test/location/foo.txt']
		    tags = ["foo", 'bar', 'baz']"#,
                StoreEntry {
                    name: "test/location/foo.txt".to_string(),
                    description: None,
                    keyword: Keyword::None,
                    entry: EntryType::FileEntry("test/location/foo.txt".to_string()),
                    tags: ["foo", "bar", "baz"]
                        .into_iter()
                        .map(str::to_string)
                        .collect(),
                    icon: None,
                    icon_type: IconType::file("test/location/foo.txt"),
                },
            ),
            (
                r#"['foo.txt']
                    description = "asdf""#,
                StoreEntry {
                    name: "foo.txt".to_string(),
                    description: Some("asdf".to_string()),
                    keyword: Keyword::None,
                    entry: EntryType::FileEntry("foo.txt".to_string()),
                    tags: [].into_iter().map(str::to_string).collect(),
                    icon: None,
                    icon_type: IconType::file("foo.txt"),
                },
            ),
            (
                r#"['foo.txt']
                    desc = "asdf""#,
                StoreEntry {
                    name: "foo.txt".to_string(),
                    description: Some("asdf".to_string()),
                    keyword: Keyword::None,
                    entry: EntryType::FileEntry("foo.txt".to_string()),
                    tags: [].into_iter().map(str::to_string).collect(),
                    icon: None,
                    icon_type: IconType::file("foo.txt"),
                },
            ),
            (
                r#"['foo.txt']
                   icon = "asdf.png""#,
                StoreEntry {
                    name: "foo.txt".to_string(),
                    description: None,
                    keyword: Keyword::None,
                    entry: EntryType::FileEntry("foo.txt".to_string()),
                    tags: [].into_iter().map(str::to_string).collect(),
                    icon: None,
                    icon_type: IconType::custom("asdf.png"),
                },
            ),
        ];

        for (toml, expected_entry) in pairs {
            let entry = parse_entry(toml);

            assert_eq!(expected_entry, entry);
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
            description: None,
            keyword: Keyword::None,
            entry: EntryType::SystemEntry("foo bar".to_string()),
            tags: ["foo", "bar", "baz"]
                .into_iter()
                .map(str::to_string)
                .collect(),
            icon: None,
            icon_type: IconType::file("foo bar"),
        };

        let entry = parse_entry(&toml);
        assert_eq!(expected_entry, entry);
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
            description: None,
            keyword: Keyword::None,
            entry: EntryType::FileEntry(dirname.to_string()),
            tags: ["foo", "bar", "baz"]
                .into_iter()
                .map(str::to_string)
                .collect(),
            icon: None,
            icon_type: IconType::file(dirname.to_string()),
        };

        let entry = parse_entry(&toml);

        assert_eq!(expected_entry, entry);
    }

    #[test]
    fn keyword_search_results() {
        let mut entry = parse_entry(
            r#"['name:%s']
                location = "file/%s""#,
        );

        let raw: Keyword = Keyword::RawKeyword(Default::default());
        let escaped: Keyword = Keyword::EscapedKeyword(Default::default());
        let none: Keyword = Keyword::None;

        let tests = [
            (&raw, "a b", "name:b", "file/b"),
            (&raw, "a B", "name:B", "file/B"),
            (&raw, "a b c", "name:b c", "file/b c"),
            (&escaped, "a b", "name:b", "file/b"),
            (&escaped, "a b c", "name:b c", "file/b%20c"),
            (&none, "a b", "name:%s", "file/%s"),
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
                format_param(test.0, test.1),
                r#"format("{}", "{}") -> "{}" failed: "#,
                test.0,
                test.1,
                test.2
            );
        }
    }

    #[test]
    fn test_paragraph_parser() {
        let succeses = [
            "",
            "test string",
            r"2

            paragraphs",
            r"    pre",
        ];

        for s in succeses {
            assert!(
                desc_to_paragraphs(s).is_some(),
                "could not parse {}, tokens are: {:?}",
                s,
                pulldown_cmark::Parser::new(s)
                    .into_iter()
                    .collect::<Vec<_>>()
            );
        }
    }

    #[test]
    fn test_keyword_icontypes_are_parsed() {
        let entry = parse_entry(
            r#"['a']
               location = 'http://example.com/%s'
               keyword = 'a'
               "#,
        );

        assert_eq!(
            entry.icon_type,
            IconType::url(url::Url::parse("http://example.com/").unwrap())
        );

        let entry = parse_entry(
            r#"['a']
               location = '%s.txt'
               keyword = 'a'
               "#,
        );

        assert_eq!(entry.icon_type, IconType::file(".txt"))
    }
}
