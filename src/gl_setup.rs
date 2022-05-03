use super::log;
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;
use wasm_bindgen::JsValue;
use web_sys::WebGl2RenderingContext as WebGl;
use web_sys::*;

use crate::app_state::MouseButton;

pub fn initialize_webgl_context() -> Result<WebGl, JsValue> {
    let window = window().unwrap();
    let document = window.document().unwrap();
    let canvas = document.get_element_by_id("rustCanvas").unwrap();
    println!("Going to  get canvas");
    let canvas: web_sys::HtmlCanvasElement = canvas.dyn_into::<web_sys::HtmlCanvasElement>()?;
    println!("Going to  get webgl");
    let gl: WebGl = canvas.get_context("webgl2")?.unwrap().dyn_into()?;

    attach_mouse_down_handler(&canvas)?;
    attach_mouse_up_handler(&canvas)?;
    attach_mouse_move_handler(&canvas)?;
    attach_mouse_scroll_handler(&canvas)?;
    Ok(gl)
}

pub fn attach_mouse_down_handler(canvas: &HtmlCanvasElement) -> Result<(), JsValue> {
    let handler = move |event: web_sys::MouseEvent| {
        let mouse_button_option = match event.button() {
            0 => Some(MouseButton::Left),
            1 => Some(MouseButton::Right),
            _ => None,
        };
        super::app_state::update_mouse_down(
            event.client_x() as f32,
            event.client_y() as f32,
            mouse_button_option,
        );
    };

    let handler = Closure::wrap(Box::new(handler) as Box<dyn FnMut(_)>);
    canvas.add_event_listener_with_callback("mousedown", handler.as_ref().unchecked_ref())?;
    handler.forget();

    Ok(())
}

pub fn attach_mouse_scroll_handler(canvas: &HtmlCanvasElement) -> Result<(), JsValue> {
    let handler = move |event: web_sys::WheelEvent| {
        let scroll = event.delta_y() as f32;
        super::app_state::scroll_to_zoom(scroll);
    };
    let handler = Closure::wrap(Box::new(handler) as Box<dyn FnMut(_)>);
    canvas.add_event_listener_with_callback("wheel", handler.as_ref().unchecked_ref())?;
    handler.forget();
    Ok(())
}

pub fn attach_mouse_up_handler(canvas: &HtmlCanvasElement) -> Result<(), JsValue> {
    let handler = move |event: web_sys::MouseEvent| {
        super::app_state::update_mouse_down(event.client_x() as f32, event.client_y() as f32, None);
    };

    let handler = Closure::wrap(Box::new(handler) as Box<dyn FnMut(_)>);
    canvas.add_event_listener_with_callback("mouseup", handler.as_ref().unchecked_ref())?;
    println!("Got to attach mouse up handler");
    handler.forget();

    Ok(())
}

pub fn attach_mouse_move_handler(canvas: &HtmlCanvasElement) -> Result<(), JsValue> {
    let handler = move |event: web_sys::MouseEvent| {
        super::app_state::update_mouse_position(event.client_x() as f32, event.client_y() as f32);
    };

    let handler = Closure::wrap(Box::new(handler) as Box<dyn FnMut(_)>);
    canvas.add_event_listener_with_callback("mousemove", handler.as_ref().unchecked_ref())?;
    println!("Got to attach mouse move handler");
    handler.forget();

    Ok(())
}
