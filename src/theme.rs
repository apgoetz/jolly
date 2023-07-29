// contains theme definition for Jolly
use crate::{platform, ui};
use iced::application;
use iced::overlay::menu;
use iced::widget::button;
use iced::widget::container;
use iced::widget::text;
use iced::widget::text_input;
use serde;
use serde::de::{self, DeserializeSeed, Deserializer, Error, IntoDeserializer, MapAccess, Visitor};
use serde::Deserialize;
use std::fmt;
use toml;

#[derive(Debug, Copy, Clone, PartialEq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum DefaultTheme {
    Light,
    Dark,
}

impl Default for DefaultTheme {
    fn default() -> Self {
        use lazy_static::lazy_static;
        // store default theme in lazy static to avoid generating it more than once
        lazy_static! {
            static ref DEFAULT_THEME: DefaultTheme =
                if dark_light::detect() == dark_light::Mode::Dark {
                    DefaultTheme::Dark
                } else {
                    DefaultTheme::Light
                };
        }
        *DEFAULT_THEME
    }
}

// themes for jolly are based on iced themes with some weird
// differences.  there is a secret implicit Default Theme that is
// deserialized from jolly.toml before the theme is.  this allows us
// to use either the dark or light theme as the "base theme" where any
// set parameters in the theme override those colors
#[derive(Debug, Clone, PartialEq)]
pub struct Theme {
    pub background_color: ui::Color,
    pub text_color: ui::Color,
    pub accent_color: ui::Color,
    pub selected_text_color: ui::Color,
}

impl Theme {
    fn palette(&self) -> iced::theme::palette::Palette {
        iced::theme::palette::Palette {
            background: self.background_color.clone().into(),
            text: self.text_color.clone().into(),
            primary: self.accent_color.clone().into(),
            success: self.accent_color.clone().into(),
            danger: self.accent_color.clone().into(),
        }
    }

    fn extended_palette(&self) -> iced::theme::palette::Extended {
        iced::theme::palette::Extended::generate(self.palette())
    }
}

impl Default for Theme {
    fn default() -> Self {
        DefaultTheme::default().into()
    }
}

impl<'de> de::DeserializeSeed<'de> for DefaultTheme {
    type Value = Theme;

    fn deserialize<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
    where
        D: Deserializer<'de>,
    {
        const FIELDS: &'static [&'static str] = &["background", "text", "primary"];
        deserializer.deserialize_struct("Theme", FIELDS, self)
    }
}

impl<'de> Visitor<'de> for DefaultTheme {
    type Value = Theme;

    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        formatter.write_str("struct Theme")
    }

    fn visit_map<V>(self, mut map: V) -> Result<Theme, V::Error>
    where
        V: MapAccess<'de>,
    {
        #[derive(Deserialize, Debug)]
        #[serde(rename_all = "lowercase")]
        enum Field {
            #[serde(rename = "background_color")]
            BackgroundColor,
            #[serde(rename = "text_color")]
            TextColor,
            #[serde(rename = "accent_color")]
            AccentColor,
            #[serde(rename = "selected_text_color")]
            SelectedTextColor,
            #[serde(other)]
            Other,
        }

        let mut theme: Self::Value = self.into();
        let mut background_visited = false;
        let mut text_visited = false;
        let mut selected_text_visited = false;
        let mut accent_visited = false;
        while let Some(key) = map.next_key()? {
            match key {
                Field::BackgroundColor => {
                    if background_visited {
                        return Err(de::Error::duplicate_field("background_color"));
                    }
                    background_visited = true;
                    theme.background_color = map.next_value()?;
                }
                Field::TextColor => {
                    if text_visited {
                        return Err(de::Error::duplicate_field("text_color"));
                    }
                    text_visited = true;
                    theme.text_color = map.next_value()?;
                }
                Field::AccentColor => {
                    if accent_visited {
                        return Err(de::Error::duplicate_field("accent_color"));
                    }
                    accent_visited = true;
                    theme.accent_color = map.next_value()?;
                }
                Field::SelectedTextColor => {
                    if selected_text_visited {
                        return Err(de::Error::duplicate_field("selected_text_color"));
                    }
                    selected_text_visited = true;
                    theme.selected_text_color = map.next_value()?;
                }
                Field::Other => {}
            }
        }
        Ok(theme)
    }
}

impl<'de> de::Deserialize<'de> for Theme {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let map = toml::Value::deserialize(deserializer)?;
        let mut map = if let toml::Value::Table(map) = map {
            map
        } else {
            return Err(D::Error::custom("table"));
        };
        let default = if let Some(theme) = map.remove("base") {
            <DefaultTheme as Deserialize>::deserialize(theme.into_deserializer())
                .map_err(D::Error::custom)?
        } else {
            Default::default()
        };

        default
            .deserialize(toml::Value::Table(map))
            .map_err(D::Error::custom)
    }
}

// convert default theme enum into appropriate jolly default theme
impl From<DefaultTheme> for Theme {
    fn from(f: DefaultTheme) -> Self {
        match f {
            DefaultTheme::Light => Theme {
                background_color: ui::Color::from_str("white"),
                text_color: ui::Color::from_str("black"),
                accent_color: platform::accent_color(),
                selected_text_color: ui::Color::from_str("white"),
            },

            DefaultTheme::Dark => Theme {
                background_color: ui::Color::from_str("#202225"),
                text_color: ui::Color::from_str("B3B3B3"),
                accent_color: platform::accent_color(),
                selected_text_color: ui::Color::from_str("black"),
            },
        }
    }
}
// text_input::StyleSheet
// menu::StyleSheet
// application::Stylesheet

impl menu::StyleSheet for Theme {
    type Style = ();

    fn appearance(&self, _style: &Self::Style) -> menu::Appearance {
        let palette = self.extended_palette();

        menu::Appearance {
            text_color: palette.background.base.text,
            background: palette.background.weak.color.into(),
            border_width: 1.0,
            border_radius: 0.0,
            border_color: palette.background.strong.color,
            selected_text_color: self.selected_text_color.clone().into(),
            selected_background: palette.primary.base.color.into(),
        }
    }
}

// Stylesheets for Jolly are copied almost exactly verbatim from the iced theme, except for 2 differences
//
// + no support for custom themes per widget (style is nil)
//
// + colors are tweaked to emphasize primary.base color in palettes
// instead of primary.strong. This is so that jolly themes can set an
// accent color that matches their window manager.
impl application::StyleSheet for Theme {
    type Style = ();

    fn appearance(&self, _style: &Self::Style) -> application::Appearance {
        let palette = self.extended_palette();
        application::Appearance {
            background_color: palette.background.base.color,
            text_color: palette.background.base.text,
        }
    }
}

impl text::StyleSheet for Theme {
    type Style = iced::theme::Text;
    fn appearance(&self, style: Self::Style) -> text::Appearance {
        let color = match style {
            iced::theme::Text::Default => Some(self.text_color.clone().into()),
            iced::theme::Text::Color(c) => Some(c),
        };
        text::Appearance { color: color }
    }
}

#[derive(Default)]
pub enum ButtonStyle {
    #[default]
    Transparent,
    Selected,
}

impl button::StyleSheet for Theme {
    type Style = ButtonStyle;
    fn active(&self, style: &Self::Style) -> button::Appearance {
        match style {
            ButtonStyle::Transparent => button::Appearance {
                shadow_offset: iced::Vector::default(),
                text_color: self.text_color.clone().into(),
                background: None,
                border_radius: 0.0,
                border_width: 0.0,
                border_color: iced_native::Color::TRANSPARENT,
            },

            ButtonStyle::Selected => {
                let accent_color: iced::Color = self.accent_color.clone().into();
                button::Appearance {
                    shadow_offset: iced::Vector::default(),
                    text_color: self.selected_text_color.clone().into(),
                    background: Some(accent_color.into()),
                    border_radius: 5.0.into(),
                    border_width: 1.0,
                    border_color: iced_native::Color::TRANSPARENT,
                }
            }
        }
    }
}

#[derive(Default)]
pub enum ContainerStyle {
    #[default]
    Transparent,
    Selected,
    Error,
}

impl container::StyleSheet for Theme {
    type Style = ContainerStyle;
    fn appearance(&self, style: &Self::Style) -> container::Appearance {
        match style {
            ContainerStyle::Transparent => {
                iced_native::Theme::default().appearance(&Default::default())
            }

            ContainerStyle::Selected => {
                let accent_color: iced::Color = self.accent_color.clone().into();
                container::Appearance {
                    text_color: Some(self.selected_text_color.clone().into()),
                    background: Some(accent_color.into()),
                    border_radius: 5.0.into(),
                    border_width: 1.0,
                    border_color: iced_native::Color::TRANSPARENT,
                }
            }

            ContainerStyle::Error => {
                let bg_color: iced::Color = self.background_color.clone().into();
                container::Appearance {
                    text_color: Some(self.text_color.clone().into()),
                    background: Some(bg_color.into()),
                    border_radius: 5.0,
                    border_width: 2.0,
                    border_color: ui::Color::from_str("#D64541").into(),
                }
            }
        }
    }
}

impl text_input::StyleSheet for Theme {
    type Style = ();

    // not used by jolly
    fn disabled_color(
        &self,
        _: &<Self as iced::widget::text_input::StyleSheet>::Style,
    ) -> iced::Color {
        todo!()
    }

    // not used by jolly
    fn disabled(
        &self,
        _: &<Self as iced::widget::text_input::StyleSheet>::Style,
    ) -> iced::widget::text_input::Appearance {
        todo!()
    }

    fn active(&self, _style: &Self::Style) -> text_input::Appearance {
        let palette = self.extended_palette();

        text_input::Appearance {
            background: palette.background.base.color.into(),
            border_radius: 2.0,
            border_width: 1.0,
            border_color: palette.background.strong.color,
            icon_color: Default::default(),
        }
    }

    fn hovered(&self, _style: &Self::Style) -> text_input::Appearance {
        let palette = self.extended_palette();

        text_input::Appearance {
            background: palette.background.base.color.into(),
            border_radius: 2.0,
            border_width: 1.0,
            border_color: palette.background.base.text,
            icon_color: Default::default(),
        }
    }

    fn focused(&self, _style: &Self::Style) -> text_input::Appearance {
        let palette = self.extended_palette();

        text_input::Appearance {
            background: palette.background.base.color.into(),
            border_radius: 2.0,
            border_width: 1.0,
            border_color: palette.primary.base.color,
            icon_color: Default::default(),
        }
    }

    fn placeholder_color(&self, _style: &Self::Style) -> iced::Color {
        let palette = self.extended_palette();

        palette.background.strong.color
    }

    fn value_color(&self, _style: &Self::Style) -> iced::Color {
        let palette = self.extended_palette();

        palette.background.base.text
    }

    fn selection_color(&self, _style: &Self::Style) -> iced::Color {
        let palette = self.extended_palette();

        palette.primary.weak.color
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn deserialize_all_fields() {
        let theme = Theme::default();

        assert_eq!(theme, toml::from_str("").unwrap());

        let custom = Theme {
            background_color: ui::Color::from_str("red"),
            text_color: ui::Color::from_str("orange"),
            accent_color: ui::Color::from_str("yellow"),
            selected_text_color: ui::Color::from_str("green"),
        };

        let toml = r#"
		      background_color = "red"
		      text_color = "orange"
		      accent_color = "yellow"
		      selected_text_color = "green"
                   "#;

        assert_eq!(custom, toml::from_str(toml).unwrap());
    }

    #[test]
    fn set_dark_theme() {
        let toml = r#"
		      base = "dark"
                   "#;
        let theme: Theme = DefaultTheme::Dark.into();

        assert_eq!(theme, toml::from_str(toml).unwrap());
    }

    #[test]
    fn set_light_theme() {
        let toml = r#"
		      base = "light"
                   "#;
        let theme: Theme = DefaultTheme::Light.into();

        assert_eq!(theme, toml::from_str(toml).unwrap());
    }

    #[test]
    fn override_custom_default() {
        let toml = r#"
		      base = "dark"
                      accent_color = "purple"
                   "#;

        let mut theme: Theme = DefaultTheme::Dark.into();

        theme.accent_color = ui::Color::from_str("purple");

        assert_eq!(theme, toml::from_str(toml).unwrap());
    }

    #[test]
    fn accent_color_used_for_theme() {
        // test that the major accent color we use actually shows up in the theme.
        // default iced theme uses tweaked colors that dont match
        use iced::overlay::menu::StyleSheet;
        use iced::widget::text_input::StyleSheet as TextStyleSheet;
        let toml = r#"
		      accent_color = "darkblue"
                   "#;

        let color: iced::Color = ui::Color::from_str("darkblue").into();

        let theme: Theme = toml::from_str(toml).unwrap();

        let menu_appearance: menu::Appearance = theme.appearance(&());
        let text_appearance: text_input::Appearance = theme.focused(&());

        assert_eq!(
            iced::Background::Color(color),
            menu_appearance.selected_background.into()
        );

        assert_eq!(color, text_appearance.border_color);
    }
}
