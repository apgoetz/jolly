// eventually the jolly main window logic will move here out of main
// but for now it will just hold settings.

use crate::{display, theme};
use csscolorparser;
use iced;
use serde;
use serde::de::value::{StrDeserializer, StringDeserializer};
use serde::Deserialize;

#[derive(serde::Deserialize, Debug, Clone, PartialEq)]
#[serde(default)]
pub struct UISettings {
    pub width: u32,

    pub theme: theme::Theme,

    #[serde(flatten)]
    pub common: InheritedSettings,

    pub search: SearchSettings,
    pub entry: display::EntrySettings,
    pub max_results: usize,
}

#[derive(serde::Deserialize, Debug, Clone, PartialEq, Default)]
#[serde(default)]
pub struct InheritedSettings {
    text_size: Option<u16>,
}

impl InheritedSettings {
    pub fn text_size(&self) -> u16 {
        match self.text_size {
            Some(t) => t,
            None => 20,
        }
    }

    pub fn propagate(&mut self, parent: &Self) {
        if self.text_size.is_none() {
            self.text_size = parent.text_size;
        }
    }
}

impl UISettings {
    // initial "fixing" of parameters
    pub fn propagate(&mut self) {
        self.entry.propagate(&self.common);
        self.search.propagate(&self.common);
    }
}

impl Default for UISettings {
    fn default() -> Self {
        Self {
            width: 800,
            theme: Default::default(),
            common: InheritedSettings::default(),
            search: SearchSettings::default(),
            entry: Default::default(),
            max_results: 5,
        }
    }
}

// theme settings for the search window at that top of the screen
#[derive(serde::Deserialize, Debug, Clone, PartialEq)]
#[serde(default)]
pub struct SearchSettings {
    pub padding: u16,
    #[serde(flatten)]
    pub common: InheritedSettings,
}

impl SearchSettings {
    pub fn starting_height(&self) -> u32 {
        (self.common.text_size() + 2 * self.padding).into()
    }

    fn propagate(&mut self, parent: &InheritedSettings) {
        self.common.propagate(parent);
    }
}

impl Default for SearchSettings {
    fn default() -> Self {
        Self {
            padding: 10,
            common: Default::default(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct Color(pub csscolorparser::Color);

impl Color {
    // panics if input is malformed. so best used for compile time colors
    pub fn from_str(s: &str) -> Self {
        Color::deserialize(StrDeserializer::<serde::de::value::Error>::new(s)).unwrap()
    }
}

// use a custom deserializer to provide more info that we dont understand a color
impl<'de> serde::Deserialize<'de> for Color {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        use serde::de::Error;

        // deserialize as string first so error message can reference the text
        let text = String::deserialize(deserializer)?;
        let error = D::Error::custom(format!("Cannot parse color `{}`", &text));

        let string_deserializer: StringDeserializer<D::Error> = StringDeserializer::new(text);

        csscolorparser::Color::deserialize(string_deserializer)
            .map(Self)
            .map_err(|_: _| error)
    }
}

impl From<Color> for iced::Color {
    fn from(value: Color) -> Self {
        Self {
            r: value.0.r as _,
            g: value.0.g as _,
            b: value.0.b as _,
            a: value.0.a as _,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn missing_inherited_use_parent() {
        let mut child = InheritedSettings::default();
        let parent = InheritedSettings {
            text_size: Some(99),
            ..Default::default()
        };

        assert_ne!(child.text_size(), parent.text_size());

        child.propagate(&parent);

        assert_eq!(child.text_size(), parent.text_size());
    }
}
