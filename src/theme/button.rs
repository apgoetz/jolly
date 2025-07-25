
use iced::{widget::button::{Catalog, Status, Style, StyleFn}, Background, Border, Color};
use super::Theme;

impl Catalog for Theme {
    type Class<'a> = StyleFn<'a, Theme>;

    fn default<'a>() -> Self::Class<'a> {
        Box::new(transparent)
    }

    fn style(&self, class: &Self::Class<'_>, status: iced::widget::button::Status) -> Style {
        class(self, status)
    }
}

pub fn transparent(theme: &Theme, _status: Status) -> Style {
    Style {
        background: None,
        text_color: theme.text_color.clone().into(),
        border: Border { color: Color::TRANSPARENT, width: 0.0, radius: 0.0.into() },
        ..Default::default()
    }
}

pub fn selected(theme: &Theme, _status: Status) -> Style {
    Style {
        background: Some(Background::Color(theme.accent_color.clone().into())),
        text_color: theme.selected_text_color.clone().into(),
        border: Border { color: Color::TRANSPARENT, width: 1.0, radius: 5.0.into() },
        ..Default::default()
    }
}

#[derive(Default)]
pub enum ButtonStyle {
    #[default]
    Transparent,
    Selected,
}
/* 
impl button::StyleSheet for Theme {
    type Style = ButtonStyle;
    fn active(&self, style: &Self::Style) -> button::Appearance {
        match style {
            ButtonStyle::Transparent => button::Appearance {
                shadow_offset: iced::Vector::default(),
                text_color: self.text_color.clone().into(),
                background: None,
                border_radius: 0.0.into(),
                border_width: 0.0,
                border_color: iced::Color::TRANSPARENT,
            },

            ButtonStyle::Selected => {
                let accent_color: iced::Color = self.accent_color.clone().into();
                button::Appearance {
                    shadow_offset: iced::Vector::default(),
                    text_color: self.selected_text_color.clone().into(),
                    background: Some(accent_color.into()),
                    border_radius: 5.0.into(),
                    border_width: 1.0,
                    border_color: iced::Color::TRANSPARENT,
                }
            }
        }
    }
} */
