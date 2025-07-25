use iced::{widget::container::{Catalog, Style, StyleFn}, Border};
use crate::ui;

pub use iced::widget::container::transparent;

use super::Theme;

impl Catalog for Theme {
    type Class<'a> = StyleFn<'a, Self>;

    fn default<'a>() -> Self::Class<'a> {
        Box::new(transparent)
    }

    fn style(&self, class: &Self::Class<'_>) -> Style {
        class(self)
    }
}

pub fn selected(theme: &Theme) -> Style {
    Style {
        text_color: Some(theme.text_color.clone().into()),
        background: Some(iced::Background::Color(theme.accent_color.clone().into())),
        border: Border{ color: iced::Color::TRANSPARENT, width: 1.0, radius: 5.0.into() },
        shadow: Default::default(),
    }
}

pub fn error(theme: &Theme) -> Style {
    Style {
        text_color: Some(theme.text_color.clone().into()),
        background: Some(iced::Background::Color(theme.background_color.clone().into())),
        border: Border{ color: ui::Color::from_str("#D64541").into(), width: 2.0, radius: 5.0.into() },
        shadow: Default::default(),
    }
}

/* 
impl container::StyleSheet for Theme {
    type Style = ContainerStyle;
    fn appearance(&self, style: &Self::Style) -> container::Appearance {
        match style {
            ContainerStyle::Transparent => iced::Theme::default().appearance(&Default::default()),

            ContainerStyle::Selected => {
                let accent_color: iced::Color = self.accent_color.clone().into();
                container::Appearance {
                    text_color: Some(self.selected_text_color.clone().into()),
                    background: Some(accent_color.into()),
                    border_radius: 5.0.into(),
                    border_width: 1.0,
                    border_color: iced::Color::TRANSPARENT,
                }
            }

            ContainerStyle::Error => {
                let bg_color: iced::Color = self.background_color.clone().into();
                container::Appearance {
                    text_color: Some(self.text_color.clone().into()),
                    background: Some(bg_color.into()),
                    border_radius: 5.0.into(),
                    border_width: 2.0,
                    border_color: ui::Color::from_str("#D64541").into(),
                }
            }
        }
    }
} */