//! The Glass style as egui widgets: a shader for the surface (blur, lens, fill, rim, highlight) and the controls on it.

// The motion table lives at `src/motion.rs` (the crate root owns `main.rs`). It is compiled here
// until `main.rs` declares `mod motion;`.
#[path = "../motion.rs"]
pub mod motion;
mod shader;
pub mod style;
mod widgets;

pub use shader::install;
pub use eframe::egui::Ui;
pub use widgets::*;
