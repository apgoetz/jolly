use iced::{executor, Element, Application, Settings, TextInput, text_input, Command, };
use iced_native::{command,window, subscription, event, keyboard};
use std::path;
use blocking;

mod store;
mod error;

// constants used to define window shape
const UI_DEFAULT_TEXT_SIZE : u16 = 20;
const UI_DEFAULT_PADDING : u16 = 10;
const UI_WIDTH : u32 = 800;
const UI_STARTING_HEIGHT : u32 = (UI_DEFAULT_TEXT_SIZE  + 2*UI_DEFAULT_PADDING) as u32;
const UI_ENDING_HEIGHT : u32 = 11*UI_STARTING_HEIGHT;
const LOGFILE_NAME : &str = "jolly.toml";

#[derive(Debug, Clone)]
enum Message {
    StoreLoaded(Result<store::Store, String>),
    SearchTextChanged(String),
    ExternalEvent(event::Event)
}

const ESCAPE_EVENT : event::Event = event::Event::Keyboard(keyboard::Event::KeyReleased {
    key_code: keyboard::KeyCode::Escape,
    modifiers: keyboard::Modifiers::empty(),
});


enum StoreLoadedState {
    Pending,
    LoadFailed(String),
    LoadSucceeded(store::Store, String)
}

impl Default for StoreLoadedState {
    fn default() -> Self {
	StoreLoadedState::Pending
    }
}


#[derive(Default)]
struct Jolly {
    searchtext: String,
    searchtextstate: text_input::State,
    should_exit: bool,
    store_state: StoreLoadedState,
}

impl Application for Jolly {

    type Message = Message;
    type Executor = executor::Default;
    type Flags = ();
    
    fn new(_flags: Self::Flags) -> (Self, Command<Self::Message>) {
	let mut jolly = Self::default();
	jolly.searchtextstate.focus();
        (jolly, Command::perform(blocking::unblock(get_store), Message::StoreLoaded))
    }

    fn title(&self) -> String {
        String::from("jolly")
    }

    fn update(&mut self, message: Self::Message) -> Command<Self::Message>{
        match message {
	    Message::SearchTextChanged(txt) if matches!(self.store_state, StoreLoadedState::LoadSucceeded(_,_)) => {
		self.searchtext = txt;
		Command::single(command::Action::Window(window::Action::Resize{width:UI_WIDTH, height: UI_ENDING_HEIGHT}))
	    }
	    Message::ExternalEvent(event::Event::Window(window::Event::FileDropped(path))) => {
		println!("{:?}", path);
		Command::none()
	    }
	    Message::ExternalEvent(e) if e == ESCAPE_EVENT => {
		println!("escape pressed");
		self.should_exit = true;
		Command::none()
	    }
	    Message::StoreLoaded(Err(err)) => {
		self.store_state = StoreLoadedState::LoadFailed(err.to_string());
		Command::none()
	    }
	    Message::StoreLoaded(Ok(store)) => {
		let msg = format!("Loaded {} entries", store.len());
		self.store_state = StoreLoadedState::LoadSucceeded(store, msg);
		Command::none()
	    }
	    _ => Command::none()
	}
	
    }

    fn should_exit(&self) -> bool {
	self.should_exit
    }
    
    fn subscription(&self) -> iced::Subscription<Message> {
	subscription::events().map(Message::ExternalEvent)
    }

    fn view(&mut self) -> Element<Self::Message> {
	use StoreLoadedState::*;
	let default_txt = match &self.store_state {
	    Pending => "Loading Message",
	    LoadFailed(msg) => msg,
	    LoadSucceeded(_,msg) => msg,
	};

        TextInput::new(&mut self.searchtextstate,
		       default_txt,
		       &self.searchtext,
	Message::SearchTextChanged).padding(UI_DEFAULT_PADDING).into()
    }
}

fn get_logfile() -> Result<path::PathBuf, error::Error> {

    let local_path = path::Path::new(LOGFILE_NAME);
    if  local_path.exists() {
	return Ok(local_path.to_path_buf());
    }

    let config_dir = dirs::config_dir().ok_or(error::Error::CustomError("Cannot Determine Config Dir".to_string()))?;
    let config_path = config_dir.join(LOGFILE_NAME);
    if config_path.exists() {
	Ok(config_path)
    } else {
	Err(error::Error::CustomError(format!("Cannot find {}", LOGFILE_NAME)))
    }

    
}

fn get_store() -> Result<store::Store, String>{
    let logfile = get_logfile().map_err(|e| e.to_string())?;
    store::load_store(logfile).map_err(error::Error::StoreError).map_err(|e| e.to_string())
}



pub fn main() -> Result<(), error::Error> {
    
    let mut settings = Settings::default();
    settings.window.size = (UI_WIDTH,UI_STARTING_HEIGHT);
    settings.default_text_size = UI_DEFAULT_TEXT_SIZE;
    Jolly::run(settings).map_err(error::Error::IcedError)
}
