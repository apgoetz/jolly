// contains logic for displaying entries
use crate::store;
use iced_native::{event, keyboard, layout, mouse, renderer, text, widget};
use std::hash::Hash;

type Message = store::StoreEntry;

#[derive(Debug)]
pub struct Entry<'a> {
    selected: bool,
    entry: &'a store::StoreEntry,
}

impl<'a> Entry<'a> {
    pub fn new(entry: &'a store::StoreEntry) -> Self {
        Entry {
            selected: false,
            entry: entry,
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
{
    fn width(&self) -> iced_native::Length {
        iced_native::Length::Fill
    }

    fn height(&self) -> iced_native::Length {
        iced_native::Length::Shrink
    }

    fn layout(&self, _renderer: &Renderer, limits: &layout::Limits) -> layout::Node {
        // add a new height restriction of how big this entry is
        let limits = limits.height(iced_native::Length::Units(40));
        //turn limits into size?
        let size = limits.resolve(iced_native::Size::ZERO);
        // turn size into node?
        let node = layout::Node::new(size);
        node
    }

    fn draw(
        &self,
        renderer: &mut Renderer,
        style: &renderer::Style,
        layout: layout::Layout<'_>,
        _cursor_position: iced_native::Point,
        viewport: &iced_native::Rectangle,
    ) {
        // viewport is rectangle covering entire UI that is being rendered
        // layout is the shape that we have  been budgeted
        let mut color = iced_native::Color::BLACK;
        if self.selected {
            let bounds = layout
                .bounds()
                .intersection(viewport)
                .unwrap_or(layout.bounds());
            renderer.fill_quad(
                renderer::Quad {
                    bounds: bounds,
                    border_radius: 5.0,
                    border_width: 1.0,
                    border_color: iced_native::Color::TRANSPARENT,
                },
                iced_native::Color::from_rgb8(92, 144, 226),
            );
            color = iced_native::Color::WHITE;
        }

        widget::text::draw(
            renderer,
            style,
            layout,
            &self.entry.name,
            Default::default(),
            None,
            Some(color),
            iced_native::alignment::Horizontal::Left,
            iced_native::alignment::Vertical::Center,
        );
    }

    fn hash_layout(&self, state: &mut iced_native::Hasher) {
        // this is cargo cult :-(
        struct Marker;
        std::any::TypeId::of::<Marker>().hash(state);
        // only thing that can affect is the entry
        self.entry.hash(state)
    }

    fn on_event(
        &mut self,
        event: event::Event,
        layout: layout::Layout<'_>,
        cursor_position: iced_native::Point,
        _renderer: &Renderer,
        _clipboard: &mut dyn iced_native::Clipboard,
        messages: &mut Vec<Message>,
    ) -> event::Status {
        match event {
            // if we are clicked on
            event::Event::Mouse(mouse::Event::ButtonReleased(button))
                if button == mouse::Button::Left =>
            {
                if layout.bounds().contains(cursor_position) {
                    messages.push(self.entry.clone());
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
                    messages.push(self.entry.clone());
                    event::Status::Captured
                } else {
                    event::Status::Ignored
                }
            }
            // somehow on linux, the numpad key is "carriage return"
            event::Event::Keyboard(keyboard::Event::CharacterReceived(c)) => {
                if (c == '\r') && self.selected {
                    messages.push(self.entry.clone());
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
{
    fn from(entry: Entry<'a>) -> iced_native::Element<'a, Message, Renderer> {
        iced_native::Element::new(entry)
    }
}
