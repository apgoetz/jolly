use iced_native::{keyboard, text, widget};

use crate::entry;
use crate::store;
use crate::ui;

const PADDING: u16 = 2;

#[derive(Default)]
pub struct SearchResults {
    entries: Vec<store::StoreEntry>,
    selected: usize,
    settings: ui::UISettings,
}

impl SearchResults {
    pub fn new<'a>(
        results: impl Iterator<Item = &'a store::StoreEntry>,
        settings: &ui::UISettings,
    ) -> Self {
        SearchResults {
            entries: results.cloned().take(settings.max_results).collect(),
            selected: 0,
            settings: settings.clone(),
        }
    }

    pub fn height(&self) -> u32 {
        let padding = PADDING as usize;
        let height = self.settings.entry.height() as usize;
        (self.entries.len() * height + padding * 2) as u32
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
        searchtext: &str,
        f: F,
    ) -> iced_native::Element<'a, Message, Renderer>
    where
        F: 'static + Fn(store::StoreEntry) -> Message,
        Renderer: 'a + text::Renderer,
        Renderer::Theme: iced::overlay::menu::StyleSheet,
        Message: 'static,
    {
        let mut column = widget::column::Column::new().padding(PADDING);
        for (i, e) in self.entries.iter().enumerate() {
            // unwrap will never panic since UI_MAX_RESULTS is const
            let entry: iced_native::Element<_, _> = {
                let e = entry::Entry::new(searchtext, e, &self.settings);
                if i == self.selected {
                    e.selected().into()
                } else {
                    e.into()
                }
            };

            column = column.push(entry);
        }
        let element: iced_native::Element<'_, _, _> = column.into();
        element.map(move |e| f(e))
    }
}
