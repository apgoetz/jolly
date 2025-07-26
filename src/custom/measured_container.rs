//! Container that determines the min size of its content, and returns
//! it to the application to allow it to adjust window size

use iced::{event, Vector};

use iced::advanced::widget::{tree, Operation, Tree};
use iced::advanced::{self, overlay};
use iced::advanced::{layout, renderer, Clipboard, Layout, Shell, Widget};
use iced::mouse;
use iced::{Element, Event, Length, Rectangle, Size};

pub struct MeasuredContainer<'a, Message, F, Theme, Renderer = iced::Renderer>
where
    F: 'static + Copy + Fn(f32, f32) -> Message,
    Renderer: iced::advanced::Renderer,
{
    content: Element<'a, Message, Theme, Renderer>,
    msg_builder: F,
}

impl<'a, Message, Renderer, Theme, F> MeasuredContainer<'a, Message, F, Theme, Renderer>
where
    F: 'static + Copy + Fn(f32, f32) -> Message,
    Renderer: iced::advanced::Renderer,
{
    /// Creates a [`MeasuredContainer`] with the given content.
    pub fn new(content: impl Into<Element<'a, Message, Theme, Renderer>>, callback: F) -> Self {
        MeasuredContainer {
            content: content.into(),
            msg_builder: callback,
        }
    }
}

#[derive(Default)]
struct State(Option<Rectangle>); // no state

impl<'a, Message, Theme, Renderer, F> Widget<Message, Theme, Renderer>
    for MeasuredContainer<'a, Message, F, Theme, Renderer>
where
    Renderer: advanced::Renderer,
    Message: Clone,
    F: 'static + Copy + Fn(f32, f32) -> Message,
{
    fn tag(&self) -> tree::Tag {
        tree::Tag::of::<State>()
    }

    fn state(&self) -> tree::State {
        tree::State::new(State::default())
    }

    fn children(&self) -> Vec<Tree> {
        vec![Tree::new(&self.content)]
    }

    fn diff(&self, tree: &mut Tree) {
        tree.diff_children(std::slice::from_ref(&self.content));
    }

    fn size(&self) -> Size<Length> {
        self.content.as_widget().size()
    }

    fn layout(
        &self,
        tree: &mut Tree,
        renderer: &Renderer,
        limits: &layout::Limits,
    ) -> layout::Node {
        self.content
            .as_widget()
            .layout(&mut tree.children[0], renderer, limits)
    }

    fn operate(
        &self,
        tree: &mut Tree,
        layout: Layout<'_>,
        renderer: &Renderer,
        operation: &mut dyn Operation,
    ) {
        self.content
            .as_widget()
            .operate(&mut tree.children[0], layout, renderer, operation);
    }

    // when we receive events, we check to see if the minimum layout
    // of the content matches the current window dimensions. If it disagrees, we inform the main application
    fn on_event(
        &mut self,
        tree: &mut Tree,
        event: Event,
        layout: Layout<'_>,
        cursor_position: mouse::Cursor,
        renderer: &Renderer,
        clipboard: &mut dyn Clipboard,
        shell: &mut Shell<'_, Message>,
        viewport: &Rectangle<f32>,
    ) -> event::Status {
        let rstate: &State = tree.state.downcast_ref();

        let (orig_width, orig_height) = match rstate.0 {
            None => (layout.bounds().width, layout.bounds().height),
            Some(r) => (r.width, r.height),
        };

        let limits = layout::Limits::new(Size::ZERO, Size::new(orig_width, f32::INFINITY));

        let bounds = self.layout(tree, renderer, &limits).bounds();
        let new_width = bounds.width;
        let new_height = bounds.height;

        let state: &mut State = tree.state.downcast_mut();

        if new_height != orig_height || new_width != orig_width {
            shell.publish((self.msg_builder)(new_width, new_height));
            state.0 = Some(bounds);
        }

        if let event::Status::Captured = self.content.as_widget_mut().on_event(
            &mut tree.children[0],
            event.clone(),
            layout,
            cursor_position,
            renderer,
            clipboard,
            shell,
            viewport,
        ) {
            return event::Status::Captured;
        }
        event::Status::Ignored
    }

    fn mouse_interaction(
        &self,
        tree: &Tree,
        layout: Layout<'_>,
        cursor_position: mouse::Cursor,
        viewport: &Rectangle,
        renderer: &Renderer,
    ) -> mouse::Interaction {
        self.content.as_widget().mouse_interaction(
            &tree.children[0],
            layout,
            cursor_position,
            viewport,
            renderer,
        )
    }

    fn draw(
        &self,
        tree: &Tree,
        renderer: &mut Renderer,
        theme: &Theme,
        renderer_style: &renderer::Style,
        layout: Layout<'_>,
        cursor_position: mouse::Cursor,
        viewport: &Rectangle,
    ) {
        self.content.as_widget().draw(
            &tree.children[0],
            renderer,
            theme,
            renderer_style,
            layout,
            cursor_position,
            viewport,
        );
    }

    fn overlay<'b>(
        &'b mut self,
        tree: &'b mut Tree,
        layout: Layout<'_>,
        renderer: &Renderer,
        translation: Vector,
    ) -> Option<overlay::Element<'b, Message, Theme, Renderer>> {
        self.content
            .as_widget_mut()
            .overlay(&mut tree.children[0], layout, renderer, translation)
    }
}

impl<'a, Message, F, Theme, Renderer> From<MeasuredContainer<'a, Message, F, Theme, Renderer>>
    for Element<'a, Message, Theme, Renderer>
where
    Message: 'a + Clone,
    Renderer: 'a + advanced::Renderer,
    Theme: 'a,
    F: 'static + Copy + Fn(f32, f32) -> Message,
{
    fn from(
        area: MeasuredContainer<'a, Message, F, Theme, Renderer>,
    ) -> Element<'a, Message, Theme, Renderer> {
        Element::new(area)
    }
}
