//! The Glass style as egui widgets: a shader for the surface (blur, lens, fill, rim, highlight) and the controls on it.

mod shader;
pub mod style;
mod widgets;

pub use shader::install;
pub use eframe::egui::Ui;
pub use widgets::*;
