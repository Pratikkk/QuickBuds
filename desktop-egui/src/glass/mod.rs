//! The Glass style as egui widgets: a shader for the surface (blur, lens, fill, rim, highlight) and the controls on it.

/// The motion table lives at the crate root (`main.rs` reads it); the widgets here use it.
pub(crate) use crate::motion;
mod shader;
pub mod style;
mod widgets;

pub use shader::install;
pub use eframe::egui::Ui;
pub use widgets::*;
