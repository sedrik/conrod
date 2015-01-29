use std::num::Float;
use std::num::ToPrimitive;
use std::num::FromPrimitive;
use color::Color;
use dimensions::Dimensions;
use label;
use mouse::Mouse;
use opengl_graphics::Gl;
use point::Point;
use rectangle;
use ui_context::{
    UIID,
    UiContext,
};
use utils::{
    clamp,
    percentage,
    value_from_perc,
};
use widget::Widget;
use vecmath::vec2_add;

/// Represents the state of the Button widget.
#[derive(PartialEq, Clone, Copy)]
pub enum State {
    Normal,
    Highlighted,
    Clicked,
}

impl State {
    /// Return the associated Rectangle state.
    fn as_rectangle_state(&self) -> rectangle::State {
        match self {
            &State::Normal => rectangle::State::Normal,
            &State::Highlighted => rectangle::State::Highlighted,
            &State::Clicked => rectangle::State::Clicked,
        }
    }
}

widget_fns!(Slider, State, Widget::Slider(State::Normal));

/// Check the current state of the slider.
fn get_new_state(is_over: bool,
                 prev: State,
                 mouse: Mouse) -> State {
    use mouse::ButtonState::{Down, Up};
    use self::State::{Normal, Highlighted, Clicked};
    match (is_over, prev, mouse.left) {
        (true,  Normal,  Down) => Normal,
        (true,  _,       Down) => Clicked,
        (true,  _,       Up)   => Highlighted,
        (false, Clicked, Down) => Clicked,
        _ => Normal,
    }
}

/// A context on which the builder pattern can be implemented.
pub struct Slider<'a, T> {
    ui_id: UIID,
    value: T,
    min: T,
    max: T,
    pos: Point,
    dim: Dimensions,
    // maybe_callback: Option<|T|:'a>,
    maybe_callback: Option<Box<FnMut(T) + 'a>>,
    maybe_color: Option<Color>,
    maybe_frame: Option<f64>,
    maybe_frame_color: Option<Color>,
    maybe_label: Option<&'a str>,
    maybe_label_color: Option<Color>,
    maybe_label_font_size: Option<u32>,
}

impl<'a, T: Float + Copy + FromPrimitive + ToPrimitive>
Slider<'a, T> {
    /// A button builder method to be implemented by the UiContext.
    pub fn new(ui_id: UIID,
              value: T, min: T, max: T) -> Slider<'a, T> {
        Slider {
            ui_id: ui_id,
            value: value,
            min: min,
            max: max,
            pos: [0.0, 0.0],
            dim: [192.0, 48.0],
            maybe_callback: None,
            maybe_color: None,
            maybe_frame: None,
            maybe_frame_color: None,
            maybe_label: None,
            maybe_label_color: None,
            maybe_label_font_size: None,
        }
    }
}

quack! {
    slider: Slider['a, T]
    get:
    set:
        fn (val: Color) { slider.maybe_color = Some(val) }
    action:
}

impl_callable!(Slider, FnMut(T), T);
impl_frameable!(Slider, T);
impl_labelable!(Slider, T);
impl_positionable!(Slider, T);
impl_shapeable!(Slider, T);

impl<'a, T: Float + Copy + FromPrimitive + ToPrimitive>
::draw::Drawable for Slider<'a, T> {
    fn draw(&mut self, uic: &mut UiContext, graphics: &mut Gl) {

        let state = *get_state(uic, self.ui_id);
        let mouse = uic.get_mouse_state();
        let is_over = rectangle::is_over(self.pos, mouse.pos, self.dim);
        let new_state = get_new_state(is_over, state, mouse);

        let frame_w = self.maybe_frame.unwrap_or(uic.theme.frame_width);
        let frame_w2 = frame_w * 2.0;
        let frame_color = self.maybe_frame_color.unwrap_or(uic.theme.frame_color);

        let is_horizontal = self.dim[0] > self.dim[1];
        let (new_value, pad_pos, pad_dim) = if is_horizontal {
            // Horizontal.
            let p = vec2_add(self.pos, [frame_w, frame_w]);
            let max_w = self.dim[0] - frame_w2;
            let w = match (is_over, state, new_state) {
                (true, State::Highlighted, State::Clicked) | (_, State::Clicked, State::Clicked)  =>
                     clamp(mouse.pos[0] - p[0], 0f64, max_w),
                _ => clamp(percentage(self.value, self.min, self.max) as f64 * max_w, 0f64, max_w),
            };
            let h = self.dim[1] - frame_w2;
            let new_value = value_from_perc((w / max_w) as f32, self.min, self.max);
            (new_value, p, [w, h])
        } else {
            // Vertical.
            let max_h = self.dim[1] - frame_w2;
            let corner = vec2_add(self.pos, [frame_w, frame_w]);
            let y_max = corner[1] + max_h;
            let (h, p) = match (is_over, state, new_state) {
                (true, State::Highlighted, State::Clicked) | (_, State::Clicked, State::Clicked) => {
                    let p = [corner[0], clamp(mouse.pos[1], corner[1], y_max)];
                    let h = clamp(max_h - (p[1] - corner[1]), 0.0, max_h);
                    (h, p)
                },
                _ => {
                    let h = clamp(percentage(self.value, self.min, self.max) as f64 * max_h, 0.0, max_h);
                    let p = [corner[0], corner[1] + max_h - h];
                    (h, p)
                },
            };
            let w = self.dim[0] - frame_w2;
            let new_value = value_from_perc((h / max_h) as f32, self.min, self.max);
            (new_value, p, [w, h])
        };

        // Callback.
        match self.maybe_callback {
            Some(ref mut callback) => {
                if self.value != new_value || match (state, new_state) {
                    (State::Highlighted, State::Clicked) | (State::Clicked, State::Highlighted) => true,
                    _ => false,
                } { (*callback)(new_value) }
            }, None => (),
        }

        // Draw.
        let rect_state = new_state.as_rectangle_state();
        let color = self.maybe_color.unwrap_or(uic.theme.shape_color);

        // Rectangle frame / backdrop.
        rectangle::draw(uic.win_w, uic.win_h, graphics, rect_state,
                        self.pos, self.dim, None, frame_color);
        // Slider rectangle.
        rectangle::draw(uic.win_w, uic.win_h, graphics, rect_state,
                        pad_pos, pad_dim, None, color);

        // If there's a label, draw it.
        if let Some(text) = self.maybe_label {
            let text_color = self.maybe_label_color.unwrap_or(uic.theme.label_color);
            let size = self.maybe_label_font_size.unwrap_or(uic.theme.font_size_medium);
            let is_horizontal = self.dim[0] > self.dim[1];
            let l_pos = if is_horizontal {
                let x = pad_pos[0] + (pad_dim[1] - size as f64) / 2.0;
                let y = pad_pos[1] + (pad_dim[1] - size as f64) / 2.0;
                [x, y]
            } else {
                let label_w = label::width(uic, size, text.as_slice());
                let x = pad_pos[0] + (pad_dim[0] - label_w) / 2.0;
                let y = pad_pos[1] + pad_dim[1] - pad_dim[0] - frame_w;
                [x, y]
            };
            // Draw the label.
            uic.draw_text(graphics, l_pos, size, text_color, text.as_slice());
        }

        set_state(uic, self.ui_id, new_state, self.pos, self.dim);

    }
}
