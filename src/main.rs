use iced::{executor, Element, Application, Settings, TextInput, text_input, Command, };
use iced_native::{command,window};





const UI_DEFAULT_TEXT_SIZE : u16 = 20;
const UI_DEFAULT_PADDING : u16 = 10;
const UI_WIDTH : u32 = 800;
const UI_STARTING_HEIGHT : u32 = (UI_DEFAULT_TEXT_SIZE  + 2*UI_DEFAULT_PADDING) as u32;
const UI_ENDING_HEIGHT : u32 = 11*UI_STARTING_HEIGHT;
#[derive(Debug, Clone)]
enum Message {
    SearchTextChanged(String),
}
#[derive(Default)]
struct Jolly {
    searchtext: String,
    searchtextstate: text_input::State
}

impl Application for Jolly {

    type Message = Message;
    type Executor = executor::Default;
    type Flags = ();
    
    fn new(_flags: ()) -> (Self, Command<Self::Message>) {
        (Self::default(), Command::none())
    }

    fn title(&self) -> String {
        String::from("jolly")
    }

    fn update(&mut self, message: Self::Message) -> Command<Self::Message>{
        match message {
	    Message::SearchTextChanged(txt) => self.searchtext = txt
	}
	Command::single(command::Action::Window(window::Action::Resize{width:UI_WIDTH, height: UI_ENDING_HEIGHT}))
    }

    fn view(&mut self) -> Element<Self::Message> {
        TextInput::new(&mut self.searchtextstate,
		       "default_txt",
		       &self.searchtext,
	Message::SearchTextChanged).padding(UI_DEFAULT_PADDING).into()
    }
}

pub fn main() -> iced::Result {
    let mut settings = Settings::default();
    settings.window.size = (UI_WIDTH,UI_STARTING_HEIGHT);
    settings.default_text_size = UI_DEFAULT_TEXT_SIZE;
    Jolly::run(settings)
}
