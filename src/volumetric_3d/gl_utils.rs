use web_sys::WebGl2RenderingContext as WebGl;
use web_sys::*;
pub(crate) trait GlUtils {
    fn get_unif_loc(
        &self,
        program: &WebGlProgram,
        location_name: &str,
    ) -> Result<WebGlUniformLocation, String>;
}

impl GlUtils for WebGl {
    fn get_unif_loc(
        &self,
        program: &WebGlProgram,
        location_name: &str,
    ) -> Result<WebGlUniformLocation, String> {
        self.get_uniform_location(program, location_name)
            .ok_or(String::from(format!(
                "Unable to get Program location with name {:}",
                location_name
            )))
    }
}
