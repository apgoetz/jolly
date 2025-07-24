use iced::{advanced, keyboard, widget};

use crate::entry;
use crate::store;
use crate::ui;

const PADDING: u16 = 2;

#[derive(Default)]
pub struct SearchResults {
    entries: Vec<entry::EntryId>,
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
    pub fn new(results: impl Iterator<Item = entry::EntryId>, settings: &ui::UISettings) -> Self {
        SearchResults {
            entries: results.take(settings.max_results).collect(),
            selected: 0,
            settings: settings.clone(),
        }
    }

    pub fn set_selection(&mut self, id: entry::EntryId) {
        if id < self.entries.len() {
            self.selected = id;
        }
    }

    pub fn selected(&self) -> Option<entry::EntryId> {
        self.entries.get(self.selected).map(|e| *e)
    }

    pub fn handle_kb(&mut self, event: keyboard::Event) {

        match event {
            keyboard::Event::KeyPressed{
                key: keyboard::Key::Named(keyboard::key::Named::ArrowUp),
                ..
            } => if self.selected > 0 {
                self.selected -= 1;
            },
            keyboard::Event::KeyPressed{
                key: keyboard::Key::Named(keyboard::key::Named::ArrowDown),
                ..
            } => if self.selected > 0 {
                let max_num = self.entries.len();
                if self.selected + 1 < max_num {
                    self.selected += 1;
                
                }
            },
            _ => {/* do nothing */}
        }
    }

    pub fn view<'a, F, Renderer>(
        &'a self,
        searchtext: &str,
        store: &'a store::Store,
        f: F,
    ) -> iced::Element<'a, crate::Message, Renderer>
    where
        F: 'static + Copy + Fn(entry::EntryId) -> crate::Message,
        Renderer: advanced::text::Renderer,
        Renderer: advanced::image::Renderer<Handle = widget::image::Handle>,
    {
        // if we dont have any entries, return an empty search results
        // (if we dont do this, the empty column will still show its
        // padding
        if self.entries.is_empty() {
            return widget::Space::with_height(0).into();
        }

        let mut column = widget::Column::new().padding(PADDING);
        for (i, e) in self.entries.iter().enumerate() {
            let entry = store.get(*e);
            // unwrap will never panic since UI_MAX_RESULTS is const
            let entry_widget =
                entry.build_entry(f, searchtext, &self.settings, i == self.selected, *e);

            let mouse_area = widget::MouseArea::new(entry_widget).on_enter(crate::Message::EntryHovered(i));


            column = column.push(mouse_area);
        }
        let element: iced::Element<'_, _, _> = column.into();
        element
    }

    pub fn entries(&self) -> &[entry::EntryId] {
        &self.entries
    }
}
