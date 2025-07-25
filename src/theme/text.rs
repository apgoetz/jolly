use super::Theme;

use iced::widget::text::{Catalog, Style, StyleFn};

impl Catalog for Theme {
    type Class<'a> = StyleFn<'a, Self>;

    fn default<'a>() -> Self::Class<'a> {
        Box::new(primary)
    }

    fn style(&self, item: &Self::Class<'_>) -> Style {
        item(self)
    }
}
pub fn primary(theme: &Theme) -> Style {
    Style {
        color: Some(theme.text_color.clone().into()),
    }
}
