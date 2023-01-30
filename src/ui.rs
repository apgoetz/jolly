// eventually the jolly main window logic will move here out of main
// but for now it will just hold settings.

use crate::{display, platform};
use csscolorparser;
use dark_light;
use iced;
use lazy_static::lazy_static;
use serde;
#[derive(serde::Deserialize, Debug, Clone, PartialEq, Copy)]
pub enum Theme {
    #[serde(alias = "light")]
    Light,
    #[serde(alias = "dark")]
    Dark,
}

impl From<Theme> for iced::Theme {
    fn from(t: Theme) -> iced::Theme {
        // determine which theme to use.
        // if your hmi is set to dark mode, use dark theme, if in auto
        match t {
            Theme::Dark => iced::Theme::Dark,
            Theme::Light => iced::Theme::Light,
        }
    }
}

#[derive(serde::Deserialize, Debug, Clone, PartialEq)]
#[serde(default)]
pub struct UISettings {
    pub width: u32,

    pub theme: Theme,

    pub accent_color: Color,

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
    pub fn propagate(&mut self) {
        self.entry.propagate(&self.common);
        self.search.propagate(&self.common);
    }
}

impl Default for UISettings {
    fn default() -> Self {
        // store default theme in lazy static to avoid generating it more than once
        lazy_static! {
            static ref DEFAULT_THEME: Theme = if dark_light::detect() == dark_light::Mode::Dark {
                Theme::Dark
            } else {
                Theme::Light
            };
        }

        Self {
            width: 800,
            theme: *DEFAULT_THEME,
            common: InheritedSettings::default(),
            search: SearchSettings::default(),
            entry: Default::default(),
            accent_color: platform::accent_color(),
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

// use a custom deserializer to provide more info that we dont understand a color
impl<'de> serde::Deserialize<'de> for Color {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        use serde::de::value::StringDeserializer;
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
