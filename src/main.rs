#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
use iced::widget::TextInput;
use iced::{executor, Application, Command, Element, Settings, Theme};
use iced_native::widget::text_input;
use iced_native::{clipboard, command, event, keyboard, subscription, widget, window};
use lazy_static;

mod config;
mod display;
mod error;
mod platform;
mod search_results;
mod settings;
mod store;
mod ui;

lazy_static::lazy_static! {
    static ref TEXT_INPUT_ID : text_input::Id = text_input::Id::unique();
}
#[derive(Debug, Clone)]
enum Message {
    SearchTextChanged(String),
    ExternalEvent(event::Event),
    EntrySelected(store::StoreEntry),
}

const ESCAPE_EVENT: keyboard::Event = keyboard::Event::KeyReleased {
    key_code: keyboard::KeyCode::Escape,
    modifiers: keyboard::Modifiers::empty(),
};

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
struct Jolly {
    searchtext: String,
    should_exit: bool,
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

    fn handle_selection(
        &mut self,
        entry: store::StoreEntry,
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
                self.should_exit = true;
                Command::none()
            }
        }
    }
}

impl Application for Jolly {
    type Message = Message;
    type Executor = executor::Default;
    type Flags = config::Config;
    type Theme = Theme;

    fn new(config: Self::Flags) -> (Self, Command<Self::Message>) {
        let mut jolly = Self::default();

        jolly.settings = config.settings;

        jolly.store_state = match config.store {
            Ok(store) => {
                let msg = format!("Loaded {} entries", store.len());
                StoreLoadedState::LoadSucceeded(store, msg)
            }
            Err(e) => StoreLoadedState::LoadFailed(e.to_string()),
        };

        (
            jolly,
            Command::batch([
                Command::single(command::Action::Window(window::Action::SetMode(
                    window::Mode::Windowed,
                ))),
                text_input::focus(TEXT_INPUT_ID.clone()),
            ]),
        )
    }

    fn title(&self) -> String {
        String::from("jolly")
    }

    fn update(&mut self, message: Self::Message) -> Command<Self::Message> {
        match message {
            Message::SearchTextChanged(txt)
                if matches!(self.store_state, StoreLoadedState::LoadSucceeded(_, _)) =>
            {
                self.searchtext = txt;
                if let Some(store) = self.store_state.store() {
                    let matches = store.find_matches(&self.searchtext).into_iter();
                    // unwrap will never panic since UI_MAX_RESULTS is const
                    self.search_results =
                        search_results::SearchResults::new(matches, &self.settings.ui);
                    Command::single(command::Action::Window(window::Action::Resize {
                        width: self.settings.ui.width as _,
                        height: self.settings.ui.search.starting_height()
                            + self.search_results.height(),
                    }))
                } else {
                    Command::none()
                }
            }
            Message::ExternalEvent(event::Event::Window(w)) if w == window::Event::Unfocused => {
                self.should_exit = true;
                Command::none()
            }
            Message::ExternalEvent(event::Event::Window(window::Event::FileDropped(path))) => {
                println!("{:?}", path);
                Command::none()
            }
            Message::ExternalEvent(event::Event::Keyboard(e)) => {
                if e == ESCAPE_EVENT {
                    self.should_exit = true;
                } else if let keyboard::Event::ModifiersChanged(m) = e {
                    self.modifiers = m;
                }
                self.search_results.handle_kb(e);
                Command::none()
            }
            Message::EntrySelected(entry) => self.handle_selection(entry),
            _ => Command::none(),
        }
    }

    fn should_exit(&self) -> bool {
        self.should_exit
    }

    fn subscription(&self) -> iced::Subscription<Message> {
        subscription::events().map(Message::ExternalEvent)
    }

    fn view(&self) -> Element<Self::Message> {
        use StoreLoadedState::*;
        let default_txt = match &self.store_state {
            Pending => "Loading Bookmarks... ",
            LoadFailed(msg) => msg,
            LoadSucceeded(_, msg) => msg,
        };

        let mut column = widget::column::Column::new();
        column = column.push(
            TextInput::new(default_txt, &self.searchtext, Message::SearchTextChanged)
                .size(self.settings.ui.search.common.text_size())
                .id(TEXT_INPUT_ID.clone())
                .padding(self.settings.ui.search.padding),
        );
        column = column.push(
            self.search_results
                .view(&self.searchtext, Message::EntrySelected),
        );
        column.into()
    }
    fn theme(&self) -> iced::Theme {
        self.settings.ui.theme.into()
    }
}

pub fn main() -> Result<(), error::Error> {
    let config = config::Config::load();
    let mut settings = Settings::default();
    settings.window.size = (
        config.settings.ui.width,
        config.settings.ui.search.starting_height(),
    );
    settings.window.decorations = false;
    settings.window.visible = false;
    settings.default_text_size = config.settings.ui.common.text_size();
    settings.flags = config;
    Jolly::run(settings).map_err(error::Error::IcedError)
}
