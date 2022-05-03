use sycamore::prelude::*;

fn main() {
    sycamore::render(|ctx| {
        view! { ctx,
             div {"Hello"}
             canvas(id = "volumetric-3d-canvas", width = 1200, height = 800)
        }
    });
}
