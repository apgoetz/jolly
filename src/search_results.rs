use iced_native::{keyboard, widget};

use crate::entry;
use crate::theme;
use crate::ui;

const PADDING: u16 = 2;

#[derive(Default)]
pub struct SearchResults {
    entries: Vec<entry::StoreEntry>,
    selected: usize,
    settings: ui::UISettings,
}

impl std::hash::Hash for SearchResults {
    fn hash<H>(&self, state: &mut H)
    where
        H: std::hash::Hasher,
    {
        self.entries.hash(state);
        self.selected.hash(state);
    }
}

impl std::cmp::PartialEq for SearchResults {
    fn eq(&self, other: &Self) -> bool {
        use std::hash::{Hash, Hasher};

        let mut s = std::collections::hash_map::DefaultHasher::new();
        self.hash(&mut s);

        let mut o = std::collections::hash_map::DefaultHasher::new();
        other.hash(&mut o);
        s.finish() == o.finish()
    }
}

impl SearchResults {
    pub fn new<'a>(
        results: impl Iterator<Item = &'a entry::StoreEntry>,
        settings: &ui::UISettings,
    ) -> Self {
        SearchResults {
            entries: results.cloned().take(settings.max_results).collect(),
            selected: 0,
            settings: settings.clone(),
        }
    }

    pub fn selected(&self) -> &entry::StoreEntry {
        &self.entries[self.selected]
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
        F: 'static + Copy + Fn(entry::StoreEntry) -> Message,
        Message: 'static + Clone,
        Renderer: iced_native::renderer::Renderer<Theme = theme::Theme> + 'a,
        Renderer: iced_native::text::Renderer,
    {
        // if we dont have any entries, return an empty search results
        // (if we dont do this, the empty column will still show its
        // padding
        if self.entries.is_empty() {
            return iced::widget::Space::with_height(0).into();
        }

        let mut column = widget::column::Column::new().padding(PADDING);
        for (i, e) in self.entries.iter().enumerate() {
            // unwrap will never panic since UI_MAX_RESULTS is const
            let entry_widget = e.build_entry(f, searchtext, &self.settings, i == self.selected);

            column = column.push(entry_widget);
        }
        let element: iced_native::Element<'_, _, _> = column.into();
        element
    }
}
