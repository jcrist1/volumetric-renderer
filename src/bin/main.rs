use sycamore::prelude::*;
use wasm_bindgen::closure::WasmClosure;
use wasm_bindgen::JsCast;
use web_sys::Event;

fn main() {
    sycamore::render(|ctx| {
        view! { ctx,
            volumetric_renderer::App {}
        }
    });
}
