//! Container that determines the min size of its content, and returns
//! it to the application to allow it to adjust window size

use iced_native::event::{self, Event};
use iced_native::layout;
use iced_native::mouse;
use iced_native::overlay;
use iced_native::renderer;
use iced_native::widget::{tree, Operation, Tree};
use iced_native::{Clipboard, Element, Layout, Length, Point, Rectangle, Shell, Widget};

pub struct MeasuredContainer<'a, Message, Renderer, F>
where
    F: 'static + Copy + Fn(u32) -> Message,
{
    content: Element<'a, Message, Renderer>,
    msg_builder: F,
}

impl<'a, Message, Renderer, F> MeasuredContainer<'a, Message, Renderer, F>
where
    F: 'static + Copy + Fn(u32) -> Message,
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
struct State; // no state

impl<'a, Message, Renderer, F> Widget<Message, Renderer>
    for MeasuredContainer<'a, Message, Renderer, F>
where
    Renderer: iced_native::Renderer,
    Message: Clone,
    F: 'static + Copy + Fn(u32) -> Message,
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

    fn width(&self) -> Length {
        self.content.as_widget().width()
    }

    fn height(&self) -> Length {
        self.content.as_widget().height()
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
        cursor_position: Point,
        renderer: &Renderer,
        clipboard: &mut dyn Clipboard,
        shell: &mut Shell<'_, Message>,
    ) -> event::Status {
        let orig_width = layout.bounds().width;
        let orig_height = layout.bounds().height.ceil() as u32;
        let limits = iced_native::layout::Limits::new(
            iced_native::Size::ZERO,
            iced_native::Size::new(orig_width, f32::INFINITY),
        );

        let new_height = self.layout(renderer, &limits).bounds().height.ceil() as u32;
        if new_height != orig_height {
            shell.publish((self.msg_builder)(new_height));
        }

        if let event::Status::Captured = self.content.as_widget_mut().on_event(
            &mut tree.children[0],
            event.clone(),
            layout,
            cursor_position,
            renderer,
            clipboard,
            shell,
        ) {
            return event::Status::Captured;
        }
        event::Status::Ignored
    }

    fn mouse_interaction(
        &self,
        tree: &Tree,
        layout: Layout<'_>,
        cursor_position: Point,
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
        cursor_position: Point,
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
    Renderer: 'a + iced_native::Renderer,
    F: 'static + Copy + Fn(u32) -> Message,
{
    fn from(area: MeasuredContainer<'a, Message, Renderer, F>) -> Element<'a, Message, Renderer> {
        Element::new(area)
    }
}
