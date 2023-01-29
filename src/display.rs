// contains logic for displaying entries
use crate::platform;
use crate::store;
use crate::ui;
use iced_native::{event, keyboard, layout, mouse, renderer, text, widget, Shell};

type Message = store::StoreEntry;

// theme settings for each shown entry result
#[derive(serde::Deserialize, Debug, Clone, PartialEq)]
#[serde(default)]
pub struct EntrySettings {
    selected_color: ui::Color,
    #[serde(flatten)]
    common: ui::InheritedSettings,
}

impl EntrySettings {
    pub fn propagate(&mut self, parent: &ui::InheritedSettings) {
        self.common.propagate(parent);
    }
    pub fn height(&self) -> u16 {
        self.common.text_size() + 10 // fixme: get rid of ugly hacked numbers
    }
}

impl Default for EntrySettings {
    fn default() -> Self {
        Self {
            selected_color: platform::accent_color(),
            common: ui::InheritedSettings::default(),
        }
    }
}

#[derive(Debug)]
pub struct Entry<'a> {
    selected: bool,
    entry: &'a store::StoreEntry,
    title: String,
    settings: EntrySettings,
}

impl<'a> Entry<'a> {
    pub fn new(searchtext: &str, entry: &'a store::StoreEntry, settings: &EntrySettings) -> Self {
        Entry {
            selected: false,
            entry: entry,
            title: entry.format_name(searchtext),
            settings: settings.clone(),
        }
    }

    pub fn selected(mut self) -> Self {
        self.selected = true;
        self
    }
}

impl<'a, Renderer> widget::Widget<Message, Renderer> for Entry<'a>
where
    Renderer: text::Renderer,
    Renderer::Theme: iced::widget::pick_list::StyleSheet,
{
    fn width(&self) -> iced_native::Length {
        iced_native::Length::Fill
    }

    fn height(&self) -> iced_native::Length {
        iced_native::Length::Shrink
    }

    fn layout(&self, _renderer: &Renderer, limits: &layout::Limits) -> layout::Node {
        // add a new height restriction of how big this entry is
        let limits = limits
            .height(iced_native::Length::Units(
                self.settings.common.text_size() + 10,
            ))
            .width(iced_native::Length::Fill);

        //turn limits into size?
        let size = limits.resolve(iced_native::Size::ZERO);

        // turn size into node?
        let node = layout::Node::new(size);
        node
    }

    fn draw(
        &self,
        _state: &widget::Tree,
        renderer: &mut Renderer,
        theme: &Renderer::Theme,
        _style: &renderer::Style,
        layout: layout::Layout<'_>,
        _cursor_position: iced_native::Point,
        viewport: &iced_native::Rectangle,
    ) {
        use iced::widget::pick_list::StyleSheet;
        // viewport is rectangle covering entire UI that is being rendered
        // layout is the shape that we have  been budgeted
        let bounds = layout
            .bounds()
            .intersection(viewport)
            .unwrap_or(layout.bounds());

        let style = if self.selected {
            theme.hovered(&Default::default())
        } else {
            theme.active(&Default::default())
        };

        
        if self.selected {
            let selected_color: iced::Color = self.settings.selected_color.clone().into();
            renderer.fill_quad(
                renderer::Quad {
                    bounds: bounds,
                    border_radius: 5.0.into(),
                    border_width: 1.0,
                    border_color: iced_native::Color::TRANSPARENT,
                },
                selected_color,
            );
        }

        renderer.fill_text(text::Text {
            content: &self.title,
            size: renderer.default_size() as f32,
            bounds: iced_native::Rectangle {
                x: bounds.x + 5.0,
                y: bounds.center_y(),
                ..bounds
            },
            color: style.text_color,
            font: Default::default(),
            horizontal_alignment: iced_native::alignment::Horizontal::Left,
            vertical_alignment: iced_native::alignment::Vertical::Center,
        });
    }

    fn on_event(
        &mut self,
        _state: &mut widget::Tree,
        event: event::Event,
        layout: layout::Layout<'_>,
        cursor_position: iced_native::Point,
        _renderer: &Renderer,
        _clipboard: &mut dyn iced_native::Clipboard,
        shell: &mut Shell<'_, Message>,
    ) -> event::Status {
        match event {
            // if we are clicked on
            event::Event::Mouse(mouse::Event::ButtonReleased(button))
                if button == mouse::Button::Left =>
            {
                if layout.bounds().contains(cursor_position) {
                    shell.publish(self.entry.clone());
                    event::Status::Captured
                } else {
                    event::Status::Ignored
                }
            }
            // if return key is pressed
            event::Event::Keyboard(keyboard::Event::KeyReleased {
                key_code: code,
                modifiers: _,
            }) => {
                if (code == keyboard::KeyCode::NumpadEnter || code == keyboard::KeyCode::Enter)
                    && self.selected
                {
                    shell.publish(self.entry.clone());
                    event::Status::Captured
                } else {
                    event::Status::Ignored
                }
            }
            // somehow on linux, the numpad key is "carriage return"
            event::Event::Keyboard(keyboard::Event::CharacterReceived(c)) => {
                if (c == '\r') && self.selected {
                    shell.publish(self.entry.clone());
                    event::Status::Captured
                } else {
                    event::Status::Ignored
                }
            }
            _ => event::Status::Ignored,
        }
    }
}

impl<'a, Renderer> From<Entry<'a>> for iced_native::Element<'a, Message, Renderer>
where
    Renderer: 'a + text::Renderer,
    Renderer::Theme: iced::widget::pick_list::StyleSheet,
{
    fn from(entry: Entry<'a>) -> iced_native::Element<'a, Message, Renderer> {
        iced_native::Element::new(entry)
    }
}
