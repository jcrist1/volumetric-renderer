use anyhow::{Context, Result};
use web_sys::WebGl2RenderingContext as WebGl;
use web_sys::*;

use super::Error;
pub(crate) trait GlUtils {
    fn get_unif_loc(
        &self,
        program: &WebGlProgram,
        location_name: &str,
    ) -> Result<WebGlUniformLocation>;
}

impl GlUtils for WebGl {
    fn get_unif_loc(
        &self,
        program: &WebGlProgram,
        location_name: &str,
    ) -> Result<WebGlUniformLocation> {
        self.get_uniform_location(program, location_name)
            .ok_or(Error::Missing)
            .context(format!(
                "Unable to get Program location with name {:}",
                location_name
            ))
            .into()
    }
}
