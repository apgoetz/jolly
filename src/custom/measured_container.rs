//! Container that determines the min size of its content, and returns
//! it to the application to allow it to adjust window size

use iced::event;

use iced::advanced::widget::{tree, Operation, Tree};
use iced::advanced::{self, overlay};
use iced::advanced::{layout, renderer, Clipboard, Layout, Shell, Widget};
use iced::mouse;
use iced::{Element, Event, Length, Rectangle, Size};

pub struct MeasuredContainer<'a, Message, Renderer, F>
where
    F: 'static + Copy + Fn(f32, f32) -> Message,
{
    content: Element<'a, Message, Renderer>,
    msg_builder: F,
}

impl<'a, Message, Renderer, F> MeasuredContainer<'a, Message, Renderer, F>
where
    F: 'static + Copy + Fn(f32, f32) -> Message,
{
    /// Creates a [`MeasuredContainer`] with the given content.
    pub fn new(content: impl Into<Element<'a, Message, Renderer>>, callback: F) -> Self {
        MeasuredContainer {
            content: content.into(),
            msg_builder: callback,
        }
    }
}

#[derive(Default)]
struct State(Option<Rectangle>); // no state

impl<'a, Message, Renderer, F> Widget<Message, Renderer>
    for MeasuredContainer<'a, Message, Renderer, F>
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
        return Size { width: self.content.as_widget().width(), height: self.content.as_widget().height() }
    }
    


    fn layout(&self, renderer: &Renderer, limits: &layout::Limits) -> layout::Node {
        self.content.as_widget().layout(renderer, limits)
    }

    fn operate(
        &self,
        tree: &mut Tree,
        layout: Layout<'_>,
        renderer: &Renderer,
        operation: &mut dyn Operation<Message>,
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
        let state: &mut State = tree.state.downcast_mut();

        let (orig_width, orig_height) = match state.0 {
            None => (layout.bounds().width, layout.bounds().height),
            Some(r) => (r.width, r.height),
        };

        let limits = layout::Limits::new(Size::ZERO, Size::new(orig_width, f32::INFINITY));

        let bounds = self.layout(renderer, &limits).bounds();
        let new_width = bounds.width;
        let new_height = bounds.height;

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
        theme: &Renderer::Theme,
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
    ) -> Option<overlay::Element<'b, Message, Renderer>> {
        self.content
            .as_widget_mut()
            .overlay(&mut tree.children[0], layout, renderer)
    }
}

impl<'a, Message, Renderer, F> From<MeasuredContainer<'a, Message, Renderer, F>>
    for Element<'a, Message, Renderer>
where
    Message: 'a + Clone,
    Renderer: 'a + advanced::Renderer,
    F: 'static + Copy + Fn(f32, f32) -> Message,
{
    fn from(area: MeasuredContainer<'a, Message, Renderer, F>) -> Element<'a, Message, Renderer> {
        Element::new(area)
    }
}
