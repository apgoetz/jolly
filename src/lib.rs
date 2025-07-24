//! Jolly is a binary crate that is not intended to be used as a
//! library. Its API is unstable and undocumented, and it only exists
//! in order to support certain integration testing and benchmarking.
//!
//! You can find documentation for the Jolly crate at its homepage,
//! [https://github.com/apgoetz/jolly](https://github.com/apgoetz/jolly)

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use ::log::trace;
use iced::widget::text::Shaping;
use iced::widget::{text_input, Container};
use iced::widget::{Text, TextInput};
use iced::{clipboard, event, keyboard, subscription, widget, window};
use iced::{executor, Application, Command, Element, Length, Renderer, Size};
use lazy_static;
use std::sync::mpsc;

pub mod cli;
pub mod config;
mod custom;
mod entry;
pub mod error;
mod icon;
mod log;
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
    EntrySelected(entry::EntryId),
    EntryHovered(entry::EntryId),
    DimensionsChanged(f32, f32),
    StartedIconWorker(mpsc::Sender<icon::IconCommand>),
    IconReceived(icon::IconType, icon::Icon),
}

#[derive(Debug)]
enum StoreLoadedState {
    Pending,
    Finished(error::Error),
    LoadSucceeded(store::Store, String),
}

impl Default for StoreLoadedState {
    fn default() -> Self {
        StoreLoadedState::Pending
    }
}

#[derive(Default)]
pub struct Jolly {
    searchtext: String,
    store_state: StoreLoadedState,
    search_results: search_results::SearchResults,
    modifiers: keyboard::Modifiers,
    settings: settings::Settings,
    icache: icon::IconCache,
    bounds: iced::Rectangle,
    focused_once: bool, // for some reason gnome defocusses
                        // the jolly window when launching, so we have to ignore
                        // defocus events until we receive a focus event.
}

impl Jolly {
    fn move_to_err(&mut self, err: error::Error) -> Command<<Jolly as Application>::Message> {
        ::log::error!("{err}");
        self.store_state = StoreLoadedState::Finished(err);
        Command::none()
    }

    fn handle_selection(&mut self, id: entry::EntryId) -> Command<<Jolly as Application>::Message> {
        // we can only continue if the store is loaded
        let store = match &self.store_state {
            StoreLoadedState::LoadSucceeded(s, _) => s,
            _ => return Command::none(),
        };

        let entry = store.get(id);

        // if the user is pressing the command key, we want to copy to
        // clipboard instead of opening the link
        if self.modifiers.command() {
            let result = entry.format_selection(&self.searchtext);
            let msg = format!("copied to clipboard: {}", &result);

            ::log::info!("{msg}");

            let cmds = [
                clipboard::write(result),
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
    type Executor = executor::Default;
    type Message = Message;
    type Theme = theme::Theme;
    type Flags = config::Config;

    fn new(config: Self::Flags) -> (Self, Command<Self::Message>) {
        let mut jolly = Self::default();

        jolly.settings = config.settings;

        jolly.bounds.width = jolly.settings.ui.width as f32;

        jolly.store_state = match config.store {
            Ok(store) => {
                let msg = format!("Loaded {} entries", store.len());

                StoreLoadedState::LoadSucceeded(store, msg)
            }
            Err(e) => {
                ::log::error!("{e}");
                StoreLoadedState::Finished(e)
            }
        };
        (
            jolly,
            Command::batch([
                window::change_mode(window::Mode::Windowed),
                text_input::focus(TEXT_INPUT_ID.clone()),
                // steal focus after startup: fixed bug on windows where it is possible to start jolly without focus
                window::gain_focus(),
            ]),
        )
    }

    fn title(&self) -> String {
        String::from("jolly")
    }

    fn update(&mut self, message: Self::Message) -> Command<Self::Message> {
        trace!("Received Message::{:?}", message);

        // first, match the messages that would cause us to quit regardless of application state
        match message {
            Message::ExternalEvent(event::Event::Keyboard(e)) => {
                if let keyboard::Event::KeyReleased {
                    key_code: key,
                    modifiers: _,
                } = e
                {
                    if key == keyboard::KeyCode::Escape {
                        return iced::window::close();
                    }
                }
            }
            Message::ExternalEvent(event::Event::Window(w)) if w == window::Event::Focused => {
                self.focused_once = true;
                return Command::none();
            }

            Message::ExternalEvent(event::Event::Window(w))
                if w == window::Event::Unfocused && self.focused_once =>
            {
                return iced::window::close();
            }

            // handle height change even if UI has failed to load
            Message::DimensionsChanged(width, height) => {
                let width = if matches!(self.store_state, StoreLoadedState::Finished(_)) {
                    width
                } else {
                    self.settings.ui.width as _
                };

                self.bounds.width = width;
                self.bounds.height = height;

                return window::resize(Size::new(width.ceil() as u32, height.ceil() as u32));
            }
            _ => (), // dont care at this point about other messages
        };

        // then, check if we are loaded. ifwe have failed to laod, we stop processing messages
        let store = match &mut self.store_state {
            StoreLoadedState::LoadSucceeded(s, _) => s,
            _ => return Command::none(),
        };

        // if we are here, we are loaded and we dont want to quit
        match message {
            Message::SearchTextChanged(txt) => {
                self.searchtext = txt;

                let matches = store.find_matches(&self.searchtext).into_iter();

                // todo: determine which entries need icons
                let new_results = search_results::SearchResults::new(matches, &self.settings.ui);

                // load icons of whatever matches are being displayed
                store.load_icons(new_results.entries(), &mut self.icache);

                self.search_results = new_results;

                Command::none()
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
                        let cmd = if let Some(id) = self.search_results.selected() {
                            self.handle_selection(id)
                        } else {
                            iced::window::close()
                        };
                        return cmd;
                    }
                }

                if keyboard::Event::CharacterReceived('\r') == e {
                    let cmd = if let Some(id) = self.search_results.selected() {
                        self.handle_selection(id)
                    } else {
                        iced::window::close()
                    };
                    return cmd;
                }

                if let keyboard::Event::ModifiersChanged(m) = e {
                    self.modifiers = m;
                }

                self.search_results.handle_kb(e);
                Command::none()
            }
            Message::EntryHovered(entry) => {
                self.search_results.set_selection(entry);
                Command::none()
            }
            Message::EntrySelected(entry) => self.handle_selection(entry),
            Message::StartedIconWorker(worker) => {
                worker
                    .send(icon::IconCommand::LoadSettings(
                        self.settings.ui.icon.clone(),
                    ))
                    .expect("Could not send message to iconworker");
                self.icache.set_cmd(worker);

                Command::none()
            }
            Message::IconReceived(it, icon) => {
                self.icache.add_icon(it, icon);

                store.load_icons(self.search_results.entries(), &mut self.icache);

                Command::none()
            }
            _ => Command::none(),
        }
    }

    fn view(&self) -> Element<'_, Message, Renderer<Self::Theme>> {
        use StoreLoadedState::*;

        let ui: Element<_, Renderer<Self::Theme>> = match &self.store_state {
            LoadSucceeded(store, msg) => widget::Column::new()
                .push(
                    TextInput::new(msg, &self.searchtext)
                        .on_input(Message::SearchTextChanged)
                        .size(self.settings.ui.search.common.text_size())
                        .id(TEXT_INPUT_ID.clone())
                        .padding(self.settings.ui.search.padding),
                )
                .push(
                    self.search_results
                        .view(&self.searchtext, store, Message::EntrySelected),
                )
                .into(),
            Pending => Text::new("Loading Bookmarks...").into(),
            Finished(err) => {
                let errtext = Text::new(err.to_string()).shaping(Shaping::Advanced);
                let style;
                let children;
                if let error::Error::FinalMessage(_) = err {
                    style = theme::ContainerStyle::Transparent;
                    children = vec![errtext.into()];
                } else {
                    style = theme::ContainerStyle::Error;
                    let title = Text::new("Oops, Jolly has encountered an Error...")
                        .style(iced::theme::Text::Color(
                            ui::Color::from_str("#D64541").into(),
                        ))
                        .size(2 * self.settings.ui.search.common.text_size());
                    children = vec![title.into(), errtext.into()];
                }

                let col = widget::Column::with_children(children).spacing(5);

                iced::widget::container::Container::new(col)
                    .style(style)
                    .padding(5)
                    .width(Length::Fill)
                    .into()
            }
        };
        widget::Container::new(ui).into()
       //custom::MeasuredContainer::new(ui, Message::DimensionsChanged).into()
    }

    fn theme(&self) -> Self::Theme {
        self.settings.ui.theme.clone()
    }

    fn subscription(&self) -> iced::Subscription<Message> {
        let channel = subscription::run(icon::icon_worker);
        let external = subscription::events().map(Message::ExternalEvent);
        subscription::Subscription::batch([channel, external].into_iter())
    }
}
