//! Dev tools: the packet log (Simple / Detailed), Clear, Export, Reconnect, Disconnect. Like the phone's: Simple
//! says in words what happened and leaves protocol steps out; Detailed shows every line with its bytes. One colour
//! per kind (`LogRow.kind`). Export saves the same lines as a sent report: no addresses or device names.

use crate::devlog::format;
use crate::session::{Cmd, LOG};
use crate::{with_app, App, DevLog, LogRow, MainWindow};
use slint::{ComponentHandle, Model, VecModel};

pub fn setup(main: &MainWindow) {
    let d = main.global::<DevLog>();
    d.set_rows(VecModel::from_slice(&[]));
    d.set_font(if cfg!(windows) { "Consolas" } else { "DejaVu Sans Mono" }.into());
    d.on_refresh(|| with_app(refresh));
    d.on_set_mode(|m| with_app(|a| { a.main.global::<DevLog>().set_mode(m); a.log_shown = 0; refresh(a); }));
    d.on_clear(|| with_app(|a| { LOG.lock().unwrap().clear(); a.log_shown = 0; refresh(a); }));
    d.on_reconnect(|| with_app(|a| { let _ = a.tx.send(Cmd::Disconnect); let _ = a.tx.send(Cmd::Connect); }));
    d.on_export(|| with_app(|a| {
        let text = crate::report::export_text(a);
        let dir = crate::report::folder();
        let secs = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map_or(0, |d| d.as_secs());
        let path = dir.join(format!("quickbuds-log-{secs}.txt"));
        let note = match std::fs::create_dir_all(&dir).and_then(|_| std::fs::write(&path, text)) {
            Ok(_) => format!("Saved to {}", path.display()),
            Err(e) => format!("Export failed: {e}"),
        };
        a.main.global::<DevLog>().set_note(note.into());
    }));
}

/// Adds the lines logged since the last call, newest on top; everything again after a mode change or Clear.
pub(crate) fn refresh(a: &mut App) {
    let d = a.main.global::<DevLog>();
    let log = LOG.lock().unwrap();
    if a.log_shown == log.len() && a.log_shown != 0 { return; }
    let rows = d.get_rows();
    let rows = rows.as_any().downcast_ref::<VecModel<LogRow>>().unwrap();
    if a.log_shown == 0 { rows.set_vec(Vec::new()); }
    for l in &log[a.log_shown..] { if let Some((text, kind)) = format(l, d.get_mode()) { rows.insert(0, LogRow { text: text.into(), kind }); } }
    a.log_shown = log.len();
}
