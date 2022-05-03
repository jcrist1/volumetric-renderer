use crate::CanvasDims;

use super::log;
use arcball::ArcballCamera;
use cgmath::{Matrix4, Vector2, Vector3};
use lazy_static::lazy_static;
use std::sync::Mutex;

lazy_static! {
    static ref APP_STATE: Mutex<AppState> = Mutex::new(AppState::new());
}

const CENTER: Vector3<f32> = Vector3::new(0.5, 0.5, 0.5);

pub fn update_dynamic_data(time: f32, canvas_height: f32, canvas_width: f32) {
    let min_height_width = canvas_height.min(canvas_width);
    let display_size = 0.9 * min_height_width;

    let mut data = APP_STATE.lock().unwrap();
    data.update_canvas(canvas_height, canvas_width);
}

pub enum MouseButton {
    Left,
    Right,
}

pub struct AppState {
    pub canvas_height: f32,
    pub canvas_width: f32,
    mouse_prev: Vector2<f32>,
    pub mouse_button: Option<MouseButton>,
    arcball: ArcballCamera<f32>,
    arcball_changed: bool,
}

impl AppState {
    fn new() -> Self {
        let mut arcball = ArcballCamera::new(CENTER, 1.0, [400.0, 600.0]);
        arcball.zoom(-1.0, 1.0);
        Self {
            canvas_height: 400.,
            canvas_width: 600.,
            mouse_prev: Vector2::new(0.0, 0.0),
            mouse_button: None,
            arcball,
            arcball_changed: false,
        }
    }

    fn update_canvas(&mut self, canvas_height: f32, canvas_width: f32) {
        self.canvas_height = canvas_height;
        self.canvas_width = canvas_width;
        self.arcball.update_screen(canvas_width, canvas_height)
    }

    fn get_canvas_dims(&self) -> CanvasDims {
        CanvasDims {
            width: self.canvas_width,
            height: self.canvas_height,
        }
    }

    fn update_mouse_down(&mut self, mouse_new: Vector2<f32>, mouse_button: Option<MouseButton>) {
        self.mouse_button = mouse_button;
        self.mouse_prev = mouse_new;
    }

    fn set_arcball_changed(&mut self, new_state: bool) {
        self.arcball_changed = new_state
    }

    fn update_mouse_pos(&mut self, mouse_new: Vector2<f32>) {
        if let Some(mouse_button) = &self.mouse_button {
            match mouse_button {
                MouseButton::Left => self.arcball.rotate(self.mouse_prev, mouse_new),
                MouseButton::Right => self.arcball.pan(mouse_new - self.mouse_prev, 1.0),
            }
            self.set_arcball_changed(true);
        };
        self.mouse_prev = mouse_new
    }

    fn scroll_to_zoom(&mut self, scroll_delta: f32) {
        self.arcball.zoom(scroll_delta, 1.0);
        self.arcball_changed = true;
    }

    fn get_arcball_data(&self) -> DrawData {
        let proj_view = self.arcball.get_mat4();
        let eye_pos = self.arcball.eye_pos();
        DrawData { proj_view, eye_pos }
    }

    fn get_arcball_changed(&self) -> bool {
        self.arcball_changed
    }
}

pub fn set_arcball_changed_to_false_after_draw() {
    let mut app_state = APP_STATE.lock().unwrap();
    app_state.set_arcball_changed(false)
}

pub struct DrawData {
    pub proj_view: Matrix4<f32>,
    pub eye_pos: Vector3<f32>,
}

pub fn should_i_draw() -> bool {
    let app_state = APP_STATE.lock().unwrap();
    app_state.get_arcball_changed()
}

pub fn get_canvas_dims() -> CanvasDims {
    let app_state = APP_STATE.lock().unwrap();
    app_state.get_canvas_dims()
}

pub fn update_canvas(CanvasDims { width, height }: &CanvasDims) {
    let mut app_state = APP_STATE.lock().unwrap();
    app_state.update_canvas(*height, *width)
}

pub fn update_mouse_down(x: f32, y: f32, mouse_button: Option<MouseButton>) {
    let mut app_state = APP_STATE.lock().unwrap();
    app_state.update_mouse_down(Vector2::new(x, y), mouse_button)
}

pub fn scroll_to_zoom(scroll: f32) {
    let mut app_state = APP_STATE.lock().unwrap();
    let adjusted_scroll = scroll / app_state.canvas_height;
    app_state.scroll_to_zoom(adjusted_scroll)
}

pub fn update_mouse_position(x: f32, y: f32) {
    let mut app_state = APP_STATE.lock().unwrap();
    app_state.update_mouse_pos(Vector2::new(x, y))
}

pub fn get_arcball_data() -> DrawData {
    let app_state = APP_STATE.lock().unwrap();
    app_state.get_arcball_data()
}
