use crate::{CanvasDims, Error, SharedMut};

use anyhow::{Context, Result};
use arcball::ArcballCamera;
use cgmath::{Matrix4, Vector2, Vector3};
use lazy_static::lazy_static;
use std::sync::Mutex;

const CENTER: Vector3<f32> = Vector3::new(0.5, 0.5, 0.5);

pub fn update_dynamic_data(
    app_state: SharedMut<AppState>,
    time: f32,
    canvas_height: f32,
    canvas_width: f32,
) {
    let min_height_width = canvas_height.min(canvas_width);
    let display_size = 0.9 * min_height_width;

    let mut data = app_state.lock().unwrap();
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
    pub density_data: Vec<u8>,
}

impl AppState {
    pub fn new() -> Self {
        let mut arcball = ArcballCamera::<f32>::new(CENTER, 1.0, [400.0, 600.0]);
        arcball.zoom(-1.0, 1.0);
        Self {
            canvas_height: 400.,
            canvas_width: 600.,
            mouse_prev: Vector2::new(0.0, 0.0),
            mouse_button: None,
            arcball,
            arcball_changed: false,
            density_data: Vec::new(),
        }
    }

    pub fn update_canvas(&mut self, canvas_height: f32, canvas_width: f32) {
        self.canvas_height = canvas_height;
        self.canvas_width = canvas_width;
        self.arcball.update_screen(canvas_width, canvas_height)
    }

    pub fn get_canvas_dims(&self) -> CanvasDims {
        CanvasDims {
            width: self.canvas_width,
            height: self.canvas_height,
        }
    }

    pub fn update_mouse_down(
        &mut self,
        mouse_new: Vector2<f32>,
        mouse_button: Option<MouseButton>,
    ) {
        self.mouse_button = mouse_button;
        self.mouse_prev = mouse_new;
    }

    pub fn set_arcball_changed(&mut self, new_state: bool) {
        self.arcball_changed = new_state
    }

    pub fn update_mouse_pos(&mut self, mouse_new: Vector2<f32>) {
        if let Some(mouse_button) = &self.mouse_button {
            match mouse_button {
                MouseButton::Left => self.arcball.rotate(self.mouse_prev, mouse_new),
                MouseButton::Right => self.arcball.pan(mouse_new - self.mouse_prev),
            }
            self.set_arcball_changed(true);
        };
        self.mouse_prev = mouse_new
    }

    pub fn scroll_to_zoom(&mut self, scroll_delta: f32) {
        let adjusted_scroll = scroll_delta / self.canvas_height;
        self.arcball.zoom(adjusted_scroll, 1.0);
        self.arcball_changed = true;
    }

    pub fn get_arcball_data(&self) -> DrawData {
        let proj_view = self.arcball.get_mat4();
        let eye_pos = self.arcball.eye_pos();
        DrawData { proj_view, eye_pos }
    }

    pub fn get_arcball_changed(&self) -> bool {
        self.arcball_changed
    }
}

pub fn set_arcball_changed_to_false_after_draw(app_state: &SharedMut<AppState>) {
    let mut app_state = app_state.lock().unwrap();
    app_state.set_arcball_changed(false)
}

pub struct DrawData {
    pub proj_view: Matrix4<f32>,
    pub eye_pos: Vector3<f32>,
}

pub fn should_i_draw(app_state: &SharedMut<AppState>) -> bool {
    let app_state = app_state.lock().unwrap();
    app_state.get_arcball_changed()
}

pub fn get_canvas_dims(app_state: &SharedMut<AppState>) -> Result<CanvasDims> {
    let app_state = app_state
        .lock()
        .map_err(Error::from)
        .context("failed to get canvas dims")?;
    Ok(app_state.get_canvas_dims())
}

pub fn get_arcball_data(app_state: &SharedMut<AppState>) -> DrawData {
    let app_state = app_state.lock().unwrap();
    app_state.get_arcball_data()
}
