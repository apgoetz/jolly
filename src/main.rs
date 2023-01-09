#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
use blocking;
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

// constants used to define window shape
const UI_DEFAULT_TEXT_SIZE: u16 = 20;
const UI_DEFAULT_PADDING: u16 = 10;
const UI_WIDTH: u32 = 800;
const UI_STARTING_HEIGHT: u32 = (UI_DEFAULT_TEXT_SIZE + 2 * UI_DEFAULT_PADDING) as u32;
const UI_MAX_RESULTS: u32 = 5;

lazy_static::lazy_static! {
    static ref TEXT_INPUT_ID : text_input::Id = text_input::Id::unique();
}
#[derive(Debug, Clone)]
enum Message {
    ConfigLoaded(Result<(settings::Settings, store::Store), String>),
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
}

impl Jolly {
    fn move_to_err(&mut self, err: error::Error) -> Command<<Jolly as Application>::Message> {
        self.store_state = StoreLoadedState::LoadFailed(err.to_string());
        self.searchtext = String::new();
        self.search_results = Default::default();
        Command::single(command::Action::Window(window::Action::Resize {
            width: UI_WIDTH,
            height: UI_STARTING_HEIGHT,
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
    type Flags = ();
    type Theme = Theme;

    fn new(_flags: Self::Flags) -> (Self, Command<Self::Message>) {
        let jolly = Self::default();
        (
            jolly,
            Command::batch([
                Command::single(command::Action::Window(window::Action::SetMode(
                    window::Mode::Windowed,
                ))),
                text_input::focus(TEXT_INPUT_ID.clone()),
                Command::perform(
                    blocking::unblock(config::load_config),
                    Message::ConfigLoaded,
                ),
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
                    let max_num = UI_MAX_RESULTS.min(matches.len().try_into().unwrap());
                    self.search_results = search_results::SearchResults::new(
                        matches.take(max_num.try_into().unwrap()),
                    );
                    Command::single(command::Action::Window(window::Action::Resize {
                        width: UI_WIDTH,
                        height: (1 + max_num) * UI_STARTING_HEIGHT,
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
            Message::ConfigLoaded(Err(err)) => {
                self.store_state = StoreLoadedState::LoadFailed(err.to_string());
                Command::none()
            }
            Message::ConfigLoaded(Ok((_, store))) => {
                let msg = format!("Loaded {} entries", store.len());
                self.store_state = StoreLoadedState::LoadSucceeded(store, msg);
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
                .id(TEXT_INPUT_ID.clone())
                .padding(UI_DEFAULT_PADDING),
        );
        column = column.push(
            self.search_results
                .view(&self.searchtext, Message::EntrySelected),
        );
        column.into()
    }
}

pub fn main() -> Result<(), error::Error> {
    let mut settings = Settings::default();
    settings.window.size = (UI_WIDTH, UI_STARTING_HEIGHT);
    settings.window.decorations = false;
    settings.window.visible = false;
    settings.default_text_size = UI_DEFAULT_TEXT_SIZE;
    Jolly::run(settings).map_err(error::Error::IcedError)
}
