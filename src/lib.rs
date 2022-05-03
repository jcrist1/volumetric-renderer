#![feature(const_evaluatable_checked)]
#![feature(array_zip)]
#![feature(array_map)]
mod app_state;
mod gl_setup;
mod matrix;
mod view;
mod volumetric_3d;

extern crate wasm_bindgen;
use app_state::update_canvas;
use gl_setup::*;
use volumetric_3d::*;
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;
use wasm_bindgen::JsValue;
use web_sys::WebGl2RenderingContext as WebGl;
use web_sys::*;
const CUBE_STRIP: [u8; 42] = [
    255, 255, 0, 0, 255, 0, 255, 255, 255, 0, 255, 255, 0, 0, 255, 0, 255, 0, 0, 0, 0, 255, 255, 0,
    255, 0, 0, 255, 255, 255, 255, 0, 255, 0, 0, 255, 255, 0, 0, 0, 0, 0,
];

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = console)]
    fn log(s: &str);
}

#[wasm_bindgen]
pub struct GlDraw(WebGl, CanvasDims);

#[wasm_bindgen]
pub struct CanvasDims {
    width: f32,
    height: f32,
}
#[wasm_bindgen]
impl GlDraw {
    #[wasm_bindgen(constructor)]
    pub fn new(canvas: &HtmlCanvasElement) -> GlDraw {
        let gl: WebGl = canvas
            .get_context(&"webgl2")
            .unwrap()
            .unwrap()
            .dyn_into::<WebGl>()
            .unwrap();

        let width = canvas.get_attribute("width").unwrap().parse().unwrap();
        let height = canvas.get_attribute("height").unwrap().parse().unwrap();
        let canvas_dims = CanvasDims { width, height };
        attach_mouse_down_handler(&canvas).unwrap();
        attach_mouse_up_handler(&canvas).unwrap();
        attach_mouse_move_handler(&canvas).unwrap();
        attach_mouse_scroll_handler(&canvas).unwrap();
        GlDraw(gl, canvas_dims)
    }

    #[wasm_bindgen]
    pub fn setup_program(&self, data_buffer: &::js_sys::Uint8Array) -> ProgramReady {
        let GlDraw(gl, canvas_dims) = self;
        let empty_state = volumetric_3d::new_empty_state(gl.clone());

        let arr = js_sys::Float32Array::new_with_length(CUBE_STRIP.len() as u32);
        arr.copy_from(
            &CUBE_STRIP
                .iter()
                .map(|x| (*x as f32) / 255.0)
                .collect::<Vec<_>>()[..],
        );
        let gl_state = empty_state.init(&arr).unwrap();

        let mut gl_state = gl_state
            .assemble_volumetric_3d_program(
                &volumetric_3d::shaders::VERT_SHADER,
                &volumetric_3d::shaders::FRAG_SHADER,
            )
            .unwrap();

        gl_state.init();
        let colormap_data: Vec<u8> = (0..256)
            .map(|i| vec![i, i, 10, 50])
            .flat_map(|x| x.into_iter())
            .map(|x| x as u8)
            .collect::<Vec<_>>();
        let gl_state = gl_state
            .build_textures(&colormap_data, &data_buffer.to_vec()[..])
            .unwrap();

        let mut program_ready = gl_state.set_volume_metadata();

        update_canvas(canvas_dims);
        program_ready.render_from_state();
        program_ready
    }
}
