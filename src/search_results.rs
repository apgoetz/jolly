use iced_native::{keyboard, text, widget};

use crate::display;
use crate::store;

#[derive(Default)]
pub struct SearchResults {
    entries: Vec<store::StoreEntry>,
    selected: usize,
}

impl SearchResults {
    pub fn new<'a>(results: impl Iterator<Item = &'a store::StoreEntry>) -> Self {
        SearchResults {
            entries: results.cloned().collect(),
            selected: 0,
        }
    }

    pub fn handle_kb(&mut self, event: keyboard::Event) {
        let code = match event {
            keyboard::Event::KeyPressed {
                key_code: code,
                modifiers: _,
            } => code,
            _ => return,
        };

        if code == keyboard::KeyCode::Up {
            if self.selected > 0 {
                self.selected -= 1;
            }
        }
        if code == keyboard::KeyCode::Down {
            let max_num = self.entries.len();
            if self.selected + 1 < max_num {
                self.selected += 1;
            }
        }
    }

    pub fn view<'a, F, Message, Renderer>(
        &'a self,
        f: F,
    ) -> iced_native::Element<'a, Message, Renderer>
    where
        F: 'static + Fn(store::StoreEntry) -> Message,
        Renderer: 'a + text::Renderer,
        Message: 'static,
    {
        let mut column = widget::column::Column::new();
        for (i, e) in self.entries.iter().enumerate() {
            // unwrap will never panic since UI_MAX_RESULTS is const
            let entry: iced_native::Element<_, _> = match i {
                i if i == self.selected => display::Entry::new(e).selected().into(),
                _ => display::Entry::new(e).into(),
            };
            column = column.push(entry);
        }
        let element: iced_native::Element<'_, _, _> = column.into();
        element.map(move |e| f(e))
    }
}
