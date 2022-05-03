mod gl_utils;
pub(crate) mod shaders;

extern crate wasm_bindgen;
use cgmath::{Matrix4, Vector3, Vector4};
use gl_utils::GlUtils;
use std::{iter::FlatMap, slice::Iter};
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;
use wasm_bindgen::JsValue;
use web_sys::WebGl2RenderingContext as WebGl;
use web_sys::*;

use crate::{
    app_state::{
        get_arcball_data, get_canvas_dims, set_arcball_changed_to_false_after_draw, should_i_draw,
        DrawData,
    },
    CanvasDims,
};

const VOLUME_X: i32 = 256;
const VOLUME_Y: i32 = 256;
const VOLUME_Z: i32 = 256;
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
    fn assign_vol_loc(&mut self, gl: &WebGl, location: i32) -> () {
        gl.uniform1i(Some(&self.volume), location);
    }

    fn assign_colormap(&mut self, gl: &WebGl, location: i32) -> () {
        gl.uniform1i(Some(&self.colormap), location);
    }

    fn assign_dt_scale(&mut self, gl: &WebGl, scale: f32) -> () {
        gl.uniform1f(Some(&self.dt_scale), scale);
    }

    fn assign_vol_dims(&mut self, gl: &WebGl, dimensions: &[i32; 3]) -> () {
        gl.uniform3iv_with_i32_array(Some(&self.vol_dims), dimensions);
    }

    fn assign_vol_scale(&mut self, gl: &WebGl, scales: &[f32; 3]) -> () {
        gl.uniform3fv_with_f32_array(Some(&self.vol_scale), scales);
    }

    fn assign_camera(&mut self, gl: &WebGl, camera_pos: &[f32; 3]) -> () {
        gl.uniform3fv_with_f32_array(Some(&self.camera_pos), camera_pos);
    }

    fn assign_proj_view(&mut self, gl: &WebGl, proj_view_data: &[f32; 16]) -> () {
        gl.uniform_matrix4fv_with_f32_array(Some(&self.proj_view), false, proj_view_data);
    }
}

pub(crate) struct ProgramCompiled<UniformLocations> {
    program: WebGlProgram,
    locations: UniformLocations,
}

impl ProgramCompiled<Volumetric3DLocations> {
    fn init(&mut self, gl: &WebGl) -> () {
        self.locations.assign_vol_loc(gl, 0);
        self.locations.assign_colormap(gl, 1);
        self.locations.assign_dt_scale(gl, 1.0);
    }
}

impl ProgramCompiledWithTextures<Volumetric3DLocations, Volumetric3DTextures> {
    fn set_volume_metadata(
        &mut self,
        gl: &WebGl,
        volume_dims: &[i32; 3],
        volume_scale: &[f32; 3],
    ) -> () {
        self.locations.assign_vol_scale(gl, volume_scale);
        self.locations.assign_vol_dims(gl, volume_dims);
    }
}

pub(crate) struct ProgramCompiledWithTextures<Locations, Textures> {
    program: WebGlProgram,
    locations: Locations,
    textures: Textures,
}
#[wasm_bindgen]
pub struct ProgramReady(
    WebGl,
    ProgramCompiledWithTextures<Volumetric3DLocations, Volumetric3DTextures>,
);

pub(crate) struct Volumetric3DTextures {
    colormap: WebGlTexture,
    volumetric: WebGlTexture,
}

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = console)]
    fn log(s: &str);
}

#[wasm_bindgen]
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

    #[wasm_bindgen]
    pub fn render_from_state(&mut self) {
        let CanvasDims { width, height } = get_canvas_dims();
        let persp_proj = cgmath::perspective(cgmath::Deg(65.0), width / height, 1.0, 200.0);

        if should_i_draw() {
            let DrawData { proj_view, eye_pos } = get_arcball_data();
            let proj_view = Matrix4::from(persp_proj) * proj_view;
            let camera_pos: [f32; 3] = [eye_pos.x, eye_pos.y, eye_pos.z];
            let mut i = 0;
            let mut arr: [f32; 16] = [0.0; 16];
            [proj_view.x, proj_view.y, proj_view.z, proj_view.w]
                .iter()
                .flat_map(|x| [x.x, x.y, x.z, x.w].iter().map(|x| *x).collect::<Vec<_>>())
                .for_each(|x| {
                    arr[i] = x;
                    i += 1
                });

            self.render(&camera_pos, &arr);
            set_arcball_changed_to_false_after_draw();
        }
    }
}

impl GlState<ProgramCompiledWithTextures<Volumetric3DLocations, Volumetric3DTextures>> {
    pub(crate) fn set_volume_metadata(self) -> ProgramReady {
        let GlState(gl, mut program_compiled_with_textures) = self;
        let vol_dims: [i32; 3] = [VOLUME_X, VOLUME_Y, VOLUME_Z];
        let vol_scale: [f32; 3] = [1.0, 1.0, 1.0];
        program_compiled_with_textures.set_volume_metadata(&gl, &vol_dims, &vol_scale);
        ProgramReady(gl, program_compiled_with_textures)
    }
}

impl GlState<ProgramCompiled<Volumetric3DLocations>> {
    pub(crate) fn init(&mut self) -> () {
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
    ) -> Result<
        GlState<ProgramCompiledWithTextures<Volumetric3DLocations, Volumetric3DTextures>>,
        String,
    > {
        let GlState(gl, program_compiled) = self;
        let ProgramCompiled { program, locations } = program_compiled;
        let colormap = gl
            .create_texture()
            .ok_or(String::from("Unable to create colormap texture"))?;
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
        .map_err(|err| {
            err.as_string()
                .unwrap_or_else(|| String::from("Couldn't map JsValue Error to stirng fro 2D tex"))
        })?;
        let volumetric = gl
            .create_texture()
            .ok_or(String::from("Couldn't create volume texture"))?;
        gl.active_texture(WebGl::TEXTURE0);
        gl.bind_texture(WebGl::TEXTURE_3D, Some(&volumetric));
        gl.tex_storage_3d(
            WebGl::TEXTURE_3D,
            1,
            WebGl::R8,
            VOLUME_X,
            VOLUME_Y,
            VOLUME_Z,
        );
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
        gl.tex_sub_image_3d_with_opt_u8_array(
            WebGl::TEXTURE_3D,
            0,
            0,
            0,
            0,
            VOLUME_X,
            VOLUME_Y,
            VOLUME_Z,
            WebGl::RED,
            WebGl::UNSIGNED_BYTE,
            Some(
                &volume_density_data
                    .iter()
                    .map(|x| x / 5)
                    .collect::<Vec<_>>()[..],
            ),
        )
        .map_err(|err| {
            err.as_string()
                .unwrap_or_else(|| String::from("Couldn't map JsValue Error to stirng"))
        })
        .unwrap();
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
    fn compile_shader<'b, T: shaders::Shader<'b>>(
        &self,
        shader: &T,
    ) -> Result<WebGlShader, String> {
        let GlState(gl, _) = &self;
        let to_compile_shader = gl
            .create_shader(shader.code())
            .ok_or(String::from("Error creating shader"))?; // todo: make better error
        gl.shader_source(&to_compile_shader, shader.source());
        gl.compile_shader(&to_compile_shader);
        let compiled_shader = to_compile_shader;
        let status = gl
            .get_shader_parameter(&compiled_shader, WebGl::COMPILE_STATUS)
            .as_bool()
            .ok_or(String::from("Compile failed. Unable to get params"))?;
        match status {
            false => {
                let error_message = gl
                    .get_shader_info_log(&compiled_shader)
                    .unwrap_or_else(|| String::from("No compiler log"));
                Err(error_message)
            }
            true => Ok(compiled_shader),
        }
    }

    pub(crate) fn assemble_volumetric_3d_program(
        self,
        vertex_shader: &shaders::VertexShader,
        fragment_shader: &shaders::FragmentShader,
    ) -> Result<GlState<ProgramCompiled<Volumetric3DLocations>>, String> {
        let compiled_vertex_shader = self.compile_shader(vertex_shader)?;
        let compiled_fragment_shader = self.compile_shader(fragment_shader)?;
        let GlState(gl, _) = self;

        let program = gl
            .create_program()
            .ok_or(String::from("Unable to create program"))?;
        gl.attach_shader(&program, &compiled_vertex_shader);
        gl.attach_shader(&program, &compiled_fragment_shader);
        gl.link_program(&program);

        let program_status = gl
            .get_program_parameter(&program, WebGl::LINK_STATUS)
            .as_bool()
            .ok_or(String::from("Failed to get program status"))?;
        if !program_status {
            return Err(String::from("Failed to attach shaders to program"));
        };
        let proj_view = gl.get_unif_loc(&program, &"proj_view")?;
        let camera_pos = gl.get_unif_loc(&program, &"eye_pos")?;
        let colormap = gl.get_unif_loc(&program, &"colormap")?;
        let dt_scale = gl.get_unif_loc(&program, &"dt_scale")?;
        let volume = gl.get_unif_loc(&program, &"volume")?;
        let vol_dims = gl.get_unif_loc(&program, &"volume_dims")?;
        let vol_scale = gl.get_unif_loc(&program, &"volume_scale")?;

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
