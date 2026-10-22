//! The resources (`res_build.rs`) and the Slint UI.

include!("res_build.rs");

fn main() {
    generate();
    slint_build::compile("ui/app.slint").unwrap();
}
