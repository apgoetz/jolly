//! Jolly is a binary crate that is not intended to be used as a
//! library. Its API is unstable and undocumented, and it only exists
//! in order to support certain integration testing and benchmarking.
//!
//! You can find documentation for the Jolly crate at its homepage,
//! [https://github.com/apgoetz/jolly](https://github.com/apgoetz/jolly)

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use iced::widget::TextInput;
use iced::{executor, Application, Command, Element, Renderer};
use iced_native::widget::text_input;
use iced_native::{clipboard, command, event, keyboard, subscription, widget, window};
use lazy_static;

pub mod config;
mod entry;
pub mod error;
mod measured_container;
mod platform;
mod search_results;
mod settings;
pub mod store;
mod theme;
mod ui;

lazy_static::lazy_static! {
    static ref TEXT_INPUT_ID : text_input::Id = text_input::Id::unique();
}
#[derive(Debug, Clone)]
pub enum Message {
    SearchTextChanged(String),
    ExternalEvent(event::Event),
    EntrySelected(entry::StoreEntry),
    HeightChanged(u32),
}

enum StoreLoadedState {
    Pending,
    LoadFailed(String),
    LoadSucceeded(store::Store, String),
}

impl Default for StoreLoadedState {
    fn default() -> Self {
        StoreLoadedState::Pending
    }
}

impl StoreLoadedState {
    fn store(&self) -> Option<&store::Store> {
        match self {
            StoreLoadedState::LoadSucceeded(s, _) => Some(s),
            _ => None,
        }
    }
}

#[derive(Default)]
pub struct Jolly {
    searchtext: String,
    store_state: StoreLoadedState,
    search_results: search_results::SearchResults,
    modifiers: keyboard::Modifiers,
    settings: settings::Settings,
}

impl Jolly {
    fn move_to_err(&mut self, err: error::Error) -> Command<<Jolly as Application>::Message> {
        self.store_state = StoreLoadedState::LoadFailed(err.to_string());
        self.searchtext = String::new();
        self.search_results = Default::default();
        Command::single(command::Action::Window(window::Action::Resize {
            width: self.settings.ui.width as _,
            height: self.settings.ui.search.starting_height(),
        }))
    }

    fn min_height_command(&self) -> Command<<Jolly as Application>::Message> {
        Command::single(command::Action::Window(window::Action::Resize {
            width: self.settings.ui.width as _,
            height: self.settings.ui.search.starting_height(),
        }))
    }

    fn handle_selection(
        &mut self,
        entry: entry::StoreEntry,
    ) -> Command<<Jolly as Application>::Message> {
        // if the user is pressing the command key, we want to copy to
        // clipboard instead of opening the link
        if self.modifiers.command() {
            let result = entry.format_selection(&self.searchtext);
            let msg = format!("copied to clipboard: {}", &result);
            let cmds = [
                Command::single(command::Action::Clipboard(clipboard::Action::Write(result))),
                self.move_to_err(error::Error::FinalMessage(msg)),
            ];
            Command::batch(cmds)
        } else {
            let result = entry.handle_selection(&self.searchtext);

            if let Err(e) = result.map_err(error::Error::StoreError) {
                self.move_to_err(e)
            } else {
                iced::window::close()
            }
        }
    }
}

impl Application for Jolly {
    type Message = Message;
    type Executor = executor::Default;
    type Flags = config::Config;
    type Theme = theme::Theme;

    fn new(config: Self::Flags) -> (Self, Command<Self::Message>) {
        let mut jolly = Self::default();

        jolly.settings = config.settings;

        jolly.store_state = match config.store {
            Ok(store) => {
                let msg = format!("Loaded {} entries", store.len());
                StoreLoadedState::LoadSucceeded(store, msg)
            }
            Err(e) => {
                println!("{:?}", e);
                StoreLoadedState::LoadFailed(e.to_string().replace("\n", "  "))
            }
        };

        (
            jolly,
            Command::batch([
                Command::single(command::Action::Window(window::Action::ChangeMode(
                    window::Mode::Windowed,
                ))),
                text_input::focus(TEXT_INPUT_ID.clone()),
                // steal focus after startup: fixed bug on windows where it is possible to start jolly without focus
                Command::single(command::Action::Window(window::Action::GainFocus)),
            ]),
        )
    }

    fn title(&self) -> String {
        String::from("jolly")
    }

    fn update(&mut self, message: Self::Message) -> Command<Self::Message> {
        match message {
            Message::HeightChanged(height) => {
                Command::single(command::Action::Window(window::Action::Resize {
                    width: self.settings.ui.width as _,
                    height: height,
                }))
            }
            Message::SearchTextChanged(txt)
                if matches!(self.store_state, StoreLoadedState::LoadSucceeded(_, _)) =>
            {
                self.searchtext = txt;
                if let Some(store) = self.store_state.store() {
                    let matches = store.find_matches(&self.searchtext).into_iter();
                    let new_results =
                        search_results::SearchResults::new(matches, &self.settings.ui);

                    if new_results != self.search_results {
                        self.search_results = new_results;
                        // since the search text changed we need to
                        // recalculate window height for the new results. So
                        // resize the window to hide the results until the new
                        // height is available
                        return self.min_height_command();
                    } else if self.searchtext.is_empty() {
                        return self.min_height_command();
                    }
                }
                Command::none()
            }
            Message::ExternalEvent(event::Event::Window(w)) if w == window::Event::Unfocused => {
                iced::window::close()
            }
            Message::ExternalEvent(event::Event::Window(window::Event::FileDropped(path))) => {
                println!("{:?}", path);
                Command::none()
            }
            Message::ExternalEvent(event::Event::Keyboard(e)) => {
                if let keyboard::Event::KeyReleased {
                    key_code: key,
                    modifiers: _,
                } = e
                {
                    if key == keyboard::KeyCode::Escape {
                        return iced::window::close();
                    } else if key == keyboard::KeyCode::NumpadEnter
                        || key == keyboard::KeyCode::Enter
                    {
                        return self.handle_selection(self.search_results.selected().clone());
                    }
                }

                if keyboard::Event::CharacterReceived('\r') == e {
                    return self.handle_selection(self.search_results.selected().clone());
                }

                if let keyboard::Event::ModifiersChanged(m) = e {
                    self.modifiers = m;
                }

                self.search_results.handle_kb(e);
                Command::none()
            }
            Message::EntrySelected(entry) => self.handle_selection(entry),
            _ => Command::none(),
        }
    }

    fn subscription(&self) -> iced::Subscription<Message> {
        subscription::events().map(Message::ExternalEvent)
    }

    fn view(&self) -> Element<'_, Message, Renderer<Self::Theme>> {
        use StoreLoadedState::*;
        let default_txt = match &self.store_state {
            Pending => "Loading Bookmarks... ",
            LoadFailed(msg) => msg,
            LoadSucceeded(_, msg) => msg,
        };

        let mut column = widget::column::Column::new();
        column = column.push(
            TextInput::new(default_txt, &self.searchtext)
                .on_input(Message::SearchTextChanged)
                .size(self.settings.ui.search.common.text_size())
                .id(TEXT_INPUT_ID.clone())
                .padding(self.settings.ui.search.padding),
        );

        column = column.push(
            self.search_results
                .view(&self.searchtext, Message::EntrySelected),
        );
        measured_container::MeasuredContainer::new(column, Message::HeightChanged).into()
    }

    fn theme(&self) -> Self::Theme {
        self.settings.ui.theme.clone()
    }
}
