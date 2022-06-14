pub mod app_state;
pub mod gl_setup;
mod matrix;
pub mod util;
mod view;
mod volumetric_3d;

extern crate wasm_bindgen;

use std::sync::{Mutex, MutexGuard, PoisonError, TryLockError};

use anyhow::{Context, Result};

use app_state::AppState;
use binding_site_search::ccp4::{CCP4Data, CCP4Error};
use gl_setup::{mouse_down_handler, mouse_move_handler, mouse_scroll_handler, mouse_up_handler};
use std::time;
use sycamore::motion::create_raf_loop;
use sycamore::prelude::*;
use sycamore::suspense::Suspense;
use util::LogErrWasm;
use volumetric_3d::*;
use wasm_bindgen::{JsCast, JsValue};
use web_sys::WebGl2RenderingContext as WebGl;
use web_sys::*;
const CUBE_STRIP: [u8; 42] = [
    255, 255, 0, 0, 255, 0, 255, 255, 255, 0, 255, 255, 0, 0, 255, 0, 255, 0, 0, 0, 0, 255, 255, 0,
    255, 0, 0, 255, 255, 255, 255, 0, 255, 0, 0, 255, 255, 0, 0, 0, 0, 0,
];

const FPS_THROTTLE_MS: time::Duration = time::Duration::from_millis(33);
pub type SharedMut<F> = std::sync::Arc<std::sync::Mutex<F>>;
pub fn shared_mut<F>(f: F) -> SharedMut<F> {
    std::sync::Arc::new(std::sync::Mutex::new(f))
}

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("no item found")]
    MissingItem,
    #[error("failed JsCast")]
    JsCast,
    //    #[error("error is JsValue")]
    //    Js(#[from] JsValue)
    //    #[error("Poisoned AppState: {0:?}")]
    //    Mutex(PoisonError<MutexGuard<AppState>>)
    #[error("Poisoned mutex error")]
    Poisoned,
    #[error("Failed request: {source}")]
    Http {
        #[from]
        source: reqwasm::Error,
    },
    #[error("Couldn't parse CCP4Data: {0}")]
    CCP4(String),
}

impl From<CCP4Error> for Error {
    fn from(CCP4Error(message): CCP4Error) -> Self {
        Error::CCP4(message)
    }
}
impl<T> From<PoisonError<MutexGuard<'_, T>>> for Error {
    fn from(_: PoisonError<MutexGuard<'_, T>>) -> Self {
        Error::Poisoned
    }
}

pub struct GlDraw(WebGl, CanvasDims);

pub struct CanvasDims {
    width: f32,
    height: f32,
}

pub fn gl_draw(event: Event, gl_draw: SharedMut<Option<GlDraw>>) -> Result<()> {
    let canvas = event
        .target()
        .ok_or(Error::MissingItem)
        .context("Failed to get even target for quantity change event")?
        .dyn_into::<HtmlCanvasElement>()
        .map_err(|_| Error::JsCast)
        .context("Failed tm convert quantity change event target to input element")?;
    web_sys::console::log_1(&format!("Canvas is {canvas:?}").into());

    let gl: WebGl = canvas
        .get_context("webgl2")
        // apparently get_context should fail with a `null` `JsValue` according
        // to https://developer.mozilla.org/en-US/docs/Web/API/HTMLCanvasElement/getContext
        .map_err(|_| Error::MissingItem)
        .context("failed to get webgl2 context")?
        .ok_or(Error::MissingItem)
        .context("no webgl2 context found")?
        .dyn_into::<WebGl>()
        .map_err(|_| Error::JsCast)
        .context(
            "failed to convert webgl2 named context into context of type WebGl2RenderingContext",
        )?;

    gl.clear_color(0.0, 0.5, 0.0, 1.0);
    // Clear the context with the newly set color. This is
    // the function call that actually does the drawing.
    gl.clear(WebGl::COLOR_BUFFER_BIT); //gl.COLOR_BUFFER_BIT);
    let width = canvas.get_attribute("width").unwrap().parse().unwrap();
    let height = canvas.get_attribute("height").unwrap().parse().unwrap();
    let canvas_dims = CanvasDims { width, height };
    let mut gl_draw = gl_draw
        .lock()
        .map_err(Error::from)
        .context("failed to lock gl_draw mutex in gl_draw")?;

    *gl_draw = Some(GlDraw(gl, canvas_dims));
    Ok(())
}

impl GlDraw {
    pub fn setup_program(
        &self,
        app_state: &SharedMut<AppState>,
        data_buffer: &[u8],
        dims: Dims,
    ) -> Result<ProgramReady> {
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
            .map(|i| vec![i, i, 10, 1])
            .flat_map(|x| x.into_iter())
            .map(|x| x as u8)
            .collect::<Vec<_>>();
        web_sys::console::log_1(&"Got here 2".into());
        let gl_state = gl_state
            .build_textures(&colormap_data, data_buffer, dims)
            .unwrap();

        let program_ready = gl_state.set_volume_metadata(dims);

        let mut app_state_ref = app_state
            .lock()
            .map_err(Error::from)
            .context("poisoned lock in gl_setup")?;
        app_state_ref.update_canvas(canvas_dims.width, canvas_dims.height);
        web_sys::console::log_1(&"Got here 4".into());
        // program_ready.render_from_state(&app_state);
        web_sys::console::log_1(&"Got here 5".into());
        Ok(program_ready)
    }
}

#[component]
pub fn App<G: Html>(ctx: Scope) -> View<G> {
    let window = web_sys::window().expect("no global `window` exists");
    let _body = window
        .document()
        .expect("failed to get document")
        .body()
        .expect("failed to get body");

    web_sys::console::log_1(&format!("window {window:?}").into());

    view! { ctx,
        div {
            Suspense {
                fallback: view! {ctx, "loading"},
                VolumetricRenderer {}
            }
        }
    }
}

async fn load_data_fut(app_state_signal: SharedMut<AppState>) -> Result<Dims> {
    let data = reqwasm::http::Request::get("data/skull_256x256x256_uint8.raw")
        .send()
        .await
        .map_err(Error::from)
        .context("Failed to get volumetric data")?
        .binary()
        .await
        .map_err(Error::from)
        .context("Failed to get volumetric data")?;
    web_sys::console::log_1(&format!("skull_data size {} {}", data.len(), 256 * 256 * 256).into());

    let data = reqwasm::http::Request::get("data/3bgf.ccp4")
        .send()
        .await
        .map_err(Error::from)
        .context("Failed to get volumetric data")?
        .binary()
        .await
        .map_err(Error::from)
        .context("Failed to get volumetric data")?;
    let molecular_data: CCP4Data<f32> = CCP4Data::from_read(&mut data.as_slice())
        .map_err(Error::from)
        .context("failed to load molecular data in load_data_fut")?;
    let x = molecular_data.header.n_cols;
    let y = molecular_data.header.n_rows;
    let z = molecular_data.header.n_sects;
    let dims = Dims { x, y, z };

    web_sys::console::log_1(&format!("Data size: {}", data.len()).into());

    let mut app_state = app_state_signal
        .lock()
        .map_err(Error::from)
        .context("App State mutex poisoned. Time to restart")?;
    let mean = molecular_data
        .data
        .iter()
        .map(|x| if x.is_normal() { x.abs() } else { 0.0 })
        .sum::<f32>()
        / (data.len() as f32);

    web_sys::console::log_1(&format!("mean value: {mean:?}").into());
    let bytes: Vec<u8> = molecular_data
        .data
        .into_iter()
        .map(|x| {
            if x.is_normal() {
                (x.abs() * 256.0 / 5.0).round() as u8
            } else {
                0
            }
        })
        .collect();
    let bytes_transform =
        bytes.iter().copied().map(|x| x as f32).sum::<f32>() / (bytes.len() as f32);
    web_sys::console::log_1(
        &format!(
            "{}, {x}, {y}, {z}, {}, {bytes_transform}",
            bytes.len(),
            x * y * z
        )
        .into(),
    );
    app_state.density_data = bytes;
    Ok(dims)
}

async fn fun(
    ctx: Scope<'_>,
    app_state: SharedMut<AppState>,
    gl_draw_signal: SharedMut<Option<GlDraw>>,
    program_ready: SharedMut<Option<ProgramReady>>,
) -> Result<()> {
    let program_ready_clone = program_ready.clone();
    let app_state_clone = app_state.clone();
    let (_, start, _) = create_raf_loop(ctx, move || {
        let len = (&app_state_clone)
            .lock()
            .expect("Failed to lock")
            .density_data
            .len();
        if let Some(pr) = &mut *program_ready_clone.clone().lock().expect("poisoned lock") {
            pr.render_from_state(&app_state_clone).log_err();
        }
        true
    });
    let dims = load_data_fut(app_state.clone()).await?;

    let density_data = app_state
        .clone()
        .lock()
        .map_err(Error::from)
        .context("failed to lock app_state in load_data")?
        .density_data
        .clone();
    web_sys::console::log_1(&"about to instantiate Uint8Array".into());
    web_sys::console::log_1(&"instantiated".into());
    if let Some(gl_draw) = gl_draw_signal
        .lock()
        .map_err(Error::from)
        .context("failed to lock gl_draw in load_data")?
        .as_ref()
    {
        web_sys::console::log_1(&"done loading image, now let's set up the program".into());
        let mut pr = gl_draw.setup_program(&app_state, density_data.as_slice(), dims)?;
        let mut pr_ref = program_ready
            .lock()
            .map_err(Error::from)
            .context("failed to lock program_ready mutex")?;
        web_sys::console::log_1(&"going to render".into());
        pr.render_from_state(&app_state)?;
        *pr_ref = Some(pr);
        web_sys::console::log_1(&"time to animate".into());
        start();
    } else {
        web_sys::console::log_1(&"no web gl context set up, so cannot set up program".into())
    }
    Ok(())
}

fn load_data(
    ctx: Scope<'_>,
    app_state: SharedMut<AppState>,
    gl_draw_signal: SharedMut<Option<GlDraw>>,
    program_ready: SharedMut<Option<ProgramReady>>,
) -> Result<()> {
    let test = gl_draw_signal
        .lock()
        .map_err(Error::from)
        .context("Poisoned gl_draw mutex in load_data")?
        .is_some();
    let message = if test {
        sycamore::futures::spawn_local_scoped(ctx, async move {
            fun(ctx, app_state, gl_draw_signal, program_ready)
                .await
                .log_err()
        });
        "is Some".to_string()
    } else {
        "is None".to_string()
    };
    web_sys::console::log_1(&message.into());
    Ok(())
}

#[component]
async fn VolumetricRenderer<G: Html>(ctx: Scope<'_>) -> View<G> {
    let app_state = AppState::new();
    let app_state = shared_mut(app_state);
    let program_ready: SharedMut<Option<ProgramReady>> = shared_mut(None);
    let shared_gl_draw = shared_mut(None);
    let app_state_ref = create_ref(ctx, app_state.clone());
    let gl_draw_signal_clone = shared_gl_draw.clone();
    let load = move |_: web_sys::Event| {
        load_data(
            ctx,
            app_state.clone(),
            gl_draw_signal_clone.clone(),
            program_ready.clone(),
        )
        .log_err()
    };
    view! { ctx,
         canvas(
             id = "volumetric-3d-canvas",
             width = 800u16,
             height = 800u16,
             on:dblclick = move |event| gl_draw(event, shared_gl_draw.clone()).log_err(),
             on:mousedown = |event| mouse_down_handler(event, app_state_ref).log_err(),
             on:mouseup = |event| mouse_up_handler(event, app_state_ref).log_err(),
             on:mousemove = |event| mouse_move_handler(event, app_state_ref).log_err(),
             on:wheel = |event| mouse_scroll_handler(event, app_state_ref).log_err(),
         ) {
             "Your browser does not seem to support
    HTML5 canvas."
         }
         div(on:click = load) {
             "CLICK ME"
         }
    }
}
