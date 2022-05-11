use anyhow::{Context, Result};
use cgmath::Vector2;
use wasm_bindgen::JsCast;
use wasm_bindgen::JsValue;
use web_sys::WebGl2RenderingContext as WebGl;
use web_sys::*;

use crate::app_state::AppState;
use crate::app_state::MouseButton;
use crate::Error;
use crate::SharedMut;

pub fn initialize_webgl_context() -> Result<WebGl, JsValue> {
    let window = window().unwrap();
    let document = window.document().unwrap();
    let canvas = document.get_element_by_id("rustCanvas").unwrap();
    println!("Going to  get canvas");
    let canvas: web_sys::HtmlCanvasElement = canvas.dyn_into::<web_sys::HtmlCanvasElement>()?;
    println!("Going to  get webgl");
    let gl: WebGl = canvas.get_context("webgl2")?.unwrap().dyn_into()?;

    //     attach_mouse_down_handler(&canvas)?;
    //     attach_mouse_up_handler(&canvas)?;
    //     attach_mouse_move_handler(&canvas)?;
    //     attach_mouse_scroll_handler(&canvas)?;
    Ok(gl)
}

pub fn mouse_down_handler(event: Event, app_state_signal: &SharedMut<AppState>) -> Result<()> {
    let mouse_event = event
        .dyn_into::<MouseEvent>()
        .map_err(|_| Error::JsCast)
        .context("Failed to get canvas in mouse down handler")?;
    web_sys::console::log_1(&"How do you do".into());
    let mouse_button_option = match mouse_event.button() {
        0 => Some(MouseButton::Left),
        1 => Some(MouseButton::Right),
        _ => None,
    };
    let mut app_state = app_state_signal
        .lock()
        .map_err(Error::from)
        .context("app state mutex is poisoned, you should restart")?;
    app_state.update_mouse_down(
        Vector2::new(mouse_event.client_x() as f32, mouse_event.client_y() as f32),
        mouse_button_option,
    );
    Ok(())
}

pub fn mouse_scroll_handler(event: Event, app_state: &SharedMut<AppState>) -> Result<()> {
    let wheel_event = event
        .dyn_into::<WheelEvent>()
        .map_err(|_| Error::JsCast)
        .context("Failed to read event as scroll event")?;
    let scroll = wheel_event.delta_y() as f32;
    let mut app_state = app_state
        .lock()
        .map_err(Error::from)
        .context("Failed to lock app_state in mouse scroll handler")?;
    app_state.scroll_to_zoom(scroll);
    Ok(())
}

pub fn mouse_up_handler(event: Event, app_state: &SharedMut<AppState>) -> Result<()> {
    let mouse_event = event
        .dyn_into::<MouseEvent>()
        .map_err(|_| Error::JsCast)
        .context("Failed to read event as mouse event in mouse up handler")?;

    let mut app_state = app_state
        .lock()
        .map_err(Error::from)
        .context("poisoned mutex in mouse up handler. Time to restart")?;
    app_state.update_mouse_down(
        Vector2::new(mouse_event.client_x() as f32, mouse_event.client_y() as f32),
        None,
    );

    Ok(())
}

pub fn mouse_move_handler(event: Event, app_state_ref: &SharedMut<AppState>) -> Result<()> {
    let mouse_event = event
        .dyn_into::<MouseEvent>()
        .map_err(|_| Error::JsCast)
        .context("Failed to get mouse event for event in mouse move handler")?;
    let mut app_state = app_state_ref
        .lock()
        .map_err(Error::from)
        .context("poisoned state mutex in mouse move handler. Time to restart")?;
    app_state.update_mouse_pos(Vector2::new(
        mouse_event.client_x() as f32,
        mouse_event.client_y() as f32,
    ));

    Ok(())
}
