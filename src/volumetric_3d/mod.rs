mod gl_utils;
pub(crate) mod shaders;

extern crate wasm_bindgen;
use std::iter::repeat;

use anyhow::{Context, Result};
use cgmath::Matrix4;
use gl_utils::GlUtils;
use wasm_bindgen::prelude::*;
use web_sys::WebGl2RenderingContext as WebGl;
use web_sys::*;

use crate::{
    app_state::{
        get_arcball_data, get_canvas_dims, set_arcball_changed_to_false_after_draw, should_i_draw,
        AppState, DrawData,
    },
    CanvasDims, SharedMut,
};

const CUBE_STRIP: [u8; 42] = [
    255, 255, 0, 0, 255, 0, 255, 255, 255, 0, 255, 255, 0, 0, 255, 0, 255, 0, 0, 0, 0, 255, 255, 0,
    255, 0, 0, 255, 255, 255, 255, 0, 255, 0, 0, 255, 255, 0, 0, 0, 0, 0,
];
//const CUBE_STRIP: [f32; 42] = [
//    1.0, 1.0, 0.0,
//    0.0, 1.0, 0.0,
//    1.0, 1.0, 1.0,
//    0.0, 1.0, 1.0,
//    0.0, 0.0, 1.0,
//    0.0, 1.0, 0.0,
//    0.0, 0.0, 0.0,
//    1.0, 1.0, 0.0,
//    1.0, 0.0, 0.0,
//    1.0, 1.0, 1.0,
//    1.0, 0.0, 1.0,
//    0.0, 0.0, 1.0,
//    1.0, 0.0, 0.0,
//    0.0, 0.0, 0.0,
//];
#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("{0}")]
    Message(String),
    #[error("Missing")]
    Missing,
}

pub(crate) struct GlState<State>(WebGl, State);

pub(crate) struct EmptyState();

pub(crate) fn new_empty_state(gl: WebGl) -> GlState<EmptyState> {
    GlState(gl, EmptyState())
}
impl GlState<EmptyState> {
    pub(crate) fn init(
        self,
        array: &js_sys::Float32Array,
    ) -> Result<GlState<VertexInitialised>, String> {
        let GlState(gl, _) = self;
        let vao = gl
            .create_vertex_array()
            .ok_or(String::from("Couldn't instantiate Vertex Array Object"))?;
        gl.bind_vertex_array(Some(&vao));

        let vbo = gl
            .create_buffer()
            .ok_or(String::from("Couldn't instantiate Buffer"))?;
        gl.bind_buffer(WebGl::ARRAY_BUFFER, Some(&vbo));
        gl.buffer_data_with_array_buffer_view(WebGl::ARRAY_BUFFER, array, WebGl::STATIC_DRAW);
        gl.enable_vertex_attrib_array(0);
        gl.vertex_attrib_pointer_with_f64(0, 3, WebGl::FLOAT, false, 0, 0.0);
        Ok(GlState(
            gl,
            VertexInitialised {
                vertex_array_object: vao,
                vertex_buffer_object: vbo,
            },
        ))
    }
}

pub(crate) struct Volumetric3DLocations {
    proj_view: WebGlUniformLocation,
    camera_pos: WebGlUniformLocation,
    colormap: WebGlUniformLocation,
    vol_dims: WebGlUniformLocation,
    volume: WebGlUniformLocation,
    vol_scale: WebGlUniformLocation,
    dt_scale: WebGlUniformLocation,
}

impl Volumetric3DLocations {
    /// This doesn't need a mutable reference, but it should
    fn assign_vol_loc(&mut self, gl: &WebGl, location: i32) {
        gl.uniform1i(Some(&self.volume), location);
    }

    fn assign_colormap(&mut self, gl: &WebGl, location: i32) {
        gl.uniform1i(Some(&self.colormap), location);
    }

    fn assign_dt_scale(&mut self, gl: &WebGl, scale: f32) {
        gl.uniform1f(Some(&self.dt_scale), scale);
    }

    fn assign_vol_dims(&mut self, gl: &WebGl, dimensions: &[i32; 3]) {
        gl.uniform3iv_with_i32_array(Some(&self.vol_dims), dimensions);
    }

    fn assign_vol_scale(&mut self, gl: &WebGl, scales: &[f32; 3]) {
        gl.uniform3fv_with_f32_array(Some(&self.vol_scale), scales);
    }

    fn assign_camera(&mut self, gl: &WebGl, camera_pos: &[f32; 3]) {
        gl.uniform3fv_with_f32_array(Some(&self.camera_pos), camera_pos);
    }

    fn assign_proj_view(&mut self, gl: &WebGl, proj_view_data: &[f32; 16]) {
        gl.uniform_matrix4fv_with_f32_array(Some(&self.proj_view), false, proj_view_data);
    }
}

pub(crate) struct ProgramCompiled<UniformLocations> {
    program: WebGlProgram,
    locations: UniformLocations,
}

impl ProgramCompiled<Volumetric3DLocations> {
    fn init(&mut self, gl: &WebGl) {
        self.locations.assign_vol_loc(gl, 0);
        self.locations.assign_colormap(gl, 1);
        self.locations.assign_dt_scale(gl, 1.0);
    }
}

impl ProgramCompiledWithTextures<Volumetric3DLocations, Volumetric3DTextures> {
    fn set_volume_metadata(&mut self, gl: &WebGl, volume_dims: &[i32; 3], volume_scale: &[f32; 3]) {
        self.locations.assign_vol_scale(gl, volume_scale);
        self.locations.assign_vol_dims(gl, volume_dims);
    }
}

pub(crate) struct ProgramCompiledWithTextures<Locations, Textures> {
    program: WebGlProgram,
    locations: Locations,
    textures: Textures,
}

pub struct ProgramReady(
    WebGl,
    ProgramCompiledWithTextures<Volumetric3DLocations, Volumetric3DTextures>,
);

pub(crate) struct Volumetric3DTextures {
    colormap: WebGlTexture,
    volumetric: WebGlTexture,
}

impl ProgramReady {
    pub(crate) fn render(&mut self, camera_pos: &[f32; 3], proj_view: &[f32; 16]) {
        let ProgramReady(
            gl,
            ProgramCompiledWithTextures {
                program: _,
                locations,
                ..
            },
        ) = self;
        gl.clear_color(1.0, 1.0, 1.0, 1.0);
        gl.clear(WebGl::COLOR_BUFFER_BIT);
        locations.assign_proj_view(gl, proj_view);
        locations.assign_camera(gl, camera_pos);
        gl.draw_arrays(WebGl::TRIANGLE_STRIP, 0, 14);
        gl.finish();
    }

    pub fn render_from_state(&mut self, app_state: &SharedMut<AppState>) -> Result<()> {
        let CanvasDims { width, height } = get_canvas_dims(app_state)?;
        let persp_proj = cgmath::perspective(cgmath::Deg(65.0), width / height, 1.0, 200.0);

        web_sys::console::log_1(&"iin render".into());
        if should_i_draw(app_state) {
            web_sys::console::log_1(&"i should draw".into());
            let DrawData { proj_view, eye_pos } = get_arcball_data(app_state);
            let proj_view = persp_proj * proj_view;
            let camera_pos: [f32; 3] = [eye_pos.x, eye_pos.y, eye_pos.z];
            let mut i = 0;
            let mut arr: [f32; 16] = [0.0; 16];
            [proj_view.x, proj_view.y, proj_view.z, proj_view.w]
                .iter()
                .flat_map(|x| vec![x.x, x.y, x.z, x.w])
                .for_each(|x| {
                    arr[i] = x;
                    i += 1
                });
            self.render(&camera_pos, &arr);
            set_arcball_changed_to_false_after_draw(app_state);
        }
        Ok(())
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Dims {
    pub x: i32,
    pub y: i32,
    pub z: i32,
}

impl GlState<ProgramCompiledWithTextures<Volumetric3DLocations, Volumetric3DTextures>> {
    pub(crate) fn set_volume_metadata(self, dims: Dims) -> ProgramReady {
        let GlState(gl, mut program_compiled_with_textures) = self;
        let vol_dims: [i32; 3] = [dims.x, dims.y, dims.z];
        let vol_scale: [f32; 3] = [1.0, 1.0, 1.0];
        program_compiled_with_textures.set_volume_metadata(&gl, &vol_dims, &vol_scale);
        ProgramReady(gl, program_compiled_with_textures)
    }
}

impl GlState<ProgramCompiled<Volumetric3DLocations>> {
    pub(crate) fn init(&mut self) {
        let GlState(gl, program_compiled) = self;

        program_compiled.init(gl);
        gl.enable(WebGl::CULL_FACE);
        gl.cull_face(WebGl::FRONT);

        gl.enable(WebGl::BLEND);
        gl.blend_func(WebGl::ONE, WebGl::ONE_MINUS_SRC_ALPHA);
    }

    pub(crate) fn build_textures(
        self,
        colormap_data: &[u8],
        volume_density_data: &[u8],
        dims: Dims,
    ) -> Result<GlState<ProgramCompiledWithTextures<Volumetric3DLocations, Volumetric3DTextures>>>
    {
        let GlState(gl, program_compiled) = self;
        let ProgramCompiled { program, locations } = program_compiled;
        let colormap = gl
            .create_texture()
            .ok_or(Error::Missing)
            .context("Unable to create colormap texture")?;
        gl.active_texture(WebGl::TEXTURE1);
        gl.bind_texture(WebGl::TEXTURE_2D, Some(&colormap));
        gl.tex_storage_2d(WebGl::TEXTURE_2D, 1, WebGl::RGBA8, 256, 1);
        gl.tex_parameteri(
            WebGl::TEXTURE_2D,
            WebGl::TEXTURE_MIN_FILTER,
            WebGl::LINEAR as i32,
        );
        gl.tex_parameteri(
            WebGl::TEXTURE_2D,
            WebGl::TEXTURE_WRAP_R as u32,
            WebGl::CLAMP_TO_EDGE as i32,
        );
        gl.tex_parameteri(
            WebGl::TEXTURE_2D,
            WebGl::TEXTURE_WRAP_S,
            WebGl::CLAMP_TO_EDGE as i32,
        );
        web_sys::console::log_1(&format!("colormap len: {}", colormap_data.len()).into());
        gl.tex_sub_image_2d_with_i32_and_i32_and_u32_and_type_and_opt_u8_array(
            WebGl::TEXTURE_2D,
            0,
            0,
            0,
            256,
            1,
            WebGl::RGBA, // See https://www.khronos.org/registry/webgl/specs/latest/2.0/#3.7.6 for info on formats
            WebGl::UNSIGNED_BYTE,
            Some(colormap_data),
        )
        .map_err(|_| Error::Message("Js".into()))
        .context("Failed to create tex_sub_image_2s")?;
        let volumetric = gl
            .create_texture()
            .ok_or(Error::Missing)
            .context("Couldn't create volume texture")?;
        gl.active_texture(WebGl::TEXTURE0);
        gl.bind_texture(WebGl::TEXTURE_3D, Some(&volumetric));
        gl.tex_storage_3d(WebGl::TEXTURE_3D, 1, WebGl::R8, dims.x, dims.y, dims.z);
        gl.tex_parameteri(
            WebGl::TEXTURE_3D,
            WebGl::TEXTURE_MIN_FILTER,
            WebGl::LINEAR as i32,
        );
        gl.tex_parameteri(
            WebGl::TEXTURE_3D,
            WebGl::TEXTURE_WRAP_R,
            WebGl::CLAMP_TO_EDGE as i32,
        );
        gl.tex_parameteri(
            WebGl::TEXTURE_3D,
            WebGl::TEXTURE_WRAP_S,
            WebGl::CLAMP_TO_EDGE as i32,
        );
        gl.tex_parameteri(
            WebGl::TEXTURE_3D,
            WebGl::TEXTURE_WRAP_T,
            WebGl::CLAMP_TO_EDGE as i32,
        );

        web_sys::console::log_1(
            &format!(
                "starting 3d {} {}",
                volume_density_data.len(),
                dims.x * dims.y * dims.z,
            )
            .into(),
        );

        let quant = 0;
        let n = 7055;
        let buffered_data = repeat(0)
            .take(quant)
            .chain(volume_density_data.iter().copied())
            .chain(repeat(0).take(n - quant))
            .collect::<Vec<_>>();
        gl.tex_sub_image_3d_with_opt_u8_array(
            WebGl::TEXTURE_3D,
            0,
            0,
            0,
            0,
            dims.x,
            dims.y,
            dims.z,
            WebGl::RED,
            WebGl::UNSIGNED_BYTE,
            Some(&buffered_data),
        )
        .map_err(|_| Error::Message("".into()))
        .context("failed tex sub image 3d")?;
        web_sys::console::log_1(&"done with 3d".into());
        let textures = Volumetric3DTextures {
            colormap,
            volumetric,
        };
        Ok(GlState(
            gl,
            ProgramCompiledWithTextures {
                program,
                locations,
                textures,
            },
        ))
    }
}

impl GlState<VertexInitialised> {
    fn compile_shader<'b, T: shaders::Shader<'b>>(&self, shader: &T) -> Result<WebGlShader> {
        let GlState(gl, _) = &self;
        let to_compile_shader = gl
            .create_shader(shader.code())
            .ok_or(Error::Missing)
            .context("Error creating shader")?;
        gl.shader_source(&to_compile_shader, shader.source());
        gl.compile_shader(&to_compile_shader);
        let compiled_shader = to_compile_shader;
        let status = gl
            .get_shader_parameter(&compiled_shader, WebGl::COMPILE_STATUS)
            .as_bool()
            .ok_or(Error::Missing)
            .context("Compile failed. Unable to get params")?;
        match status {
            false => {
                let error_message = gl
                    .get_shader_info_log(&compiled_shader)
                    .unwrap_or_else(|| String::from("No compiler log"));
                Err(Error::Message(error_message).into())
            }
            true => Ok(compiled_shader),
        }
    }

    pub(crate) fn assemble_volumetric_3d_program(
        self,
        vertex_shader: &shaders::VertexShader,
        fragment_shader: &shaders::FragmentShader,
    ) -> Result<GlState<ProgramCompiled<Volumetric3DLocations>>> {
        let compiled_vertex_shader = self.compile_shader(vertex_shader)?;
        let compiled_fragment_shader = self.compile_shader(fragment_shader)?;
        let GlState(gl, _) = self;

        let program = gl
            .create_program()
            .ok_or(Error::Missing)
            .context("Unable to create program")?;
        gl.attach_shader(&program, &compiled_vertex_shader);
        gl.attach_shader(&program, &compiled_fragment_shader);
        gl.link_program(&program);

        let program_status = gl
            .get_program_parameter(&program, WebGl::LINK_STATUS)
            .as_bool()
            .ok_or(Error::Missing)
            .context("Failed to get program status")?;
        if !program_status {
            return Err(Error::Message("Failed to attach shaders to program".to_string()).into());
        };
        let proj_view = gl.get_unif_loc(&program, "proj_view")?;
        let camera_pos = gl.get_unif_loc(&program, "eye_pos")?;
        let colormap = gl.get_unif_loc(&program, "colormap")?;
        let dt_scale = gl.get_unif_loc(&program, "dt_scale")?;
        let volume = gl.get_unif_loc(&program, "volume")?;
        let vol_dims = gl.get_unif_loc(&program, "volume_dims")?;
        let vol_scale = gl.get_unif_loc(&program, "volume_scale")?;

        gl.use_program(Some(&program));

        let locations = Volumetric3DLocations {
            proj_view,
            colormap,
            camera_pos,
            volume,
            vol_dims,
            vol_scale,
            dt_scale,
        };

        let state = ProgramCompiled { program, locations };
        Ok(GlState(gl, state))
    }
}

pub struct VertexInitialised {
    vertex_array_object: WebGlVertexArrayObject,
    vertex_buffer_object: WebGlBuffer,
}
