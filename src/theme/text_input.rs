use iced::{widget::text_input::{Catalog, Status, Style, StyleFn}, Border};
use super::Theme;

impl Catalog for Theme {
    type Class<'a> = StyleFn<'a, Self>;

    fn default<'a>() -> Self::Class<'a> {
        Box::new(search)
    }

    fn style(&self, class: &Self::Class<'_>, status: Status) -> Style {
        class(&self, status)
    }
}

pub fn search(theme: &Theme, status:Status) -> Style {
    let palette = theme.extended_palette();
    let bordercolor =
    match status {
        Status::Active => palette.background.strong.color,
        Status::Hovered => palette.background.base.text,
        Status::Focused => palette.primary.base.color,
        Status::Disabled => todo!(),
    };
    Style{ background: iced::Background::Color(palette.background.base.color.into()), 
            border: Border{ color: bordercolor, width: 1.0, radius: 2.0.into() }, 
            icon: Default::default(), 
            placeholder: palette.background.strong.color, 
            value: palette.background.base.text, 
            selection: palette.primary.weak.color }

}

/* 
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
            border_radius: 2.0.into(),
            border_width: 1.0,
            border_color: palette.background.strong.color,
            icon_color: Default::default(),
        }
    }

    fn hovered(&self, _style: &Self::Style) -> text_input::Appearance {
        let palette = self.extended_palette();

        text_input::Appearance {
            background: palette.background.base.color.into(),
            border_radius: 2.0.into(),
            border_width: 1.0,
            border_color: palette.background.base.text,
            icon_color: Default::default(),
        }
    }

    fn focused(&self, _style: &Self::Style) -> text_input::Appearance {
        let palette = self.extended_palette();

        text_input::Appearance {
            background: palette.background.base.color.into(),
            border_radius: 2.0.into(),
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
} */