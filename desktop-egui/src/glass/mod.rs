//! The Glass style as egui widgets: a shader for the surface (blur, lens, fill, rim, highlight) and the controls on it.

mod shader;
mod widgets;

pub use shader::install;
pub use widgets::*;
