//! Motion: the timing table, the easing curves and the animation primitives of DESIGN section 5.
//!
//! Every duration and curve in this file comes from the design notes section 5.1 or from a
//! developer capture of the animations. A call site must use a name from this table. Do not put a
//! duration at a call site. A value that the design notes mark `unknown` stays `unknown` here; the report
//! names the file and the symbol to measure.
//!
//! # API
//!
//! - `tween(ctx, id, target, up, down)` returns a value in 0..1. The first call snaps to `target`
//!   (DESIGN row 7 and row 15). A later change slides: `up` when the target grows, `down` when it
//!   falls. An equal target emits nothing and stops the repaint.
//! - `fade(ctx, id, on, timing)` is `tween` for a boolean.
//! - `run(ctx, id, timing)` returns the eased progress of a one-shot animation. It starts on the
//!   first call. It starts again after a gap of two frames without a call.
//! - `sheet_enter` returns the alpha and the scale of a dialog or a sheet.
//! - `page_enter`, `page_below`, `page_return` and `page_leave` return the alpha, the vertical
//!   offset in points and the scale of a page change. Pass the window height to the offset.
//! - `set_reduced_motion(true)` turns every animation into an instant state change.
//!
//! A running animation requests a repaint. A resting control does not request a repaint.
//!
//! # Wiring that `main.rs` still needs
//!
//! - Call `set_reduced_motion(...)` once. egui 0.37 has no reduced-motion input, so the default is
//!   `false` until `main.rs` reads the OS preference.
//! - Wrap the page switch with `page_leave` on the old page, `page_below` on the page under an
//!   opening page, `page_enter` on the new page and `page_return` on the page that comes back.
//! - The sheet exit is UNKNOWN in DESIGN row 18. Measure it before you add the exit.

//! # Unknown values
//!
//! - The switch thumb curve. DESIGN row 14 says "platform switch easing" and gives no formula.
//!   Measure `Switch.setFromBuds` and record it in `SWITCH_SLIDE`.
//! - The sheet exit. DESIGN row 18 gives no duration and no curve. The capture
//!   `anim-sheet-open-close.mp4` shows 59 to 95 ms of visible change. Measure
//!   `BottomSheetDialog.present` before you add an exit animation.

use eframe::egui::{Context, Id};
use std::sync::atomic::{AtomicBool, Ordering};

/// The easing curves of DESIGN section 5.1.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Curve {
    Linear,
    /// `1 - (1 - t)^factor`. Factor 1 is the plain decelerate curve.
    Decelerate(f32),
    /// `cubic-bezier(0.4, 0.0, 1.0, 1.0)`.
    AccelerateCubic,
    /// `cubic-bezier(0.0, 0.0, 0.2, 1.0)`.
    DecelerateCubic,
    /// `(t-1)^2 * ((tension+1)*(t-1) + tension) + 1`. The value passes 1.0 on the way.
    Overshoot(f32),
}

impl Curve {
    /// The eased value at time `t` in 0..1. An overshoot curve can return a value above 1.0.
    pub fn at(self, t: f32) -> f32 {
        let t = t.clamp(0.0, 1.0);
        match self {
            Curve::Linear => t,
            Curve::Decelerate(factor) => 1.0 - (1.0 - t).powf(factor),
            Curve::AccelerateCubic => bezier(0.4, 0.0, 1.0, 1.0, t),
            Curve::DecelerateCubic => bezier(0.0, 0.0, 0.2, 1.0, t),
            Curve::Overshoot(k) => (t - 1.0).powi(2) * ((k + 1.0) * (t - 1.0) + k) + 1.0,
        }
    }
}

/// A cubic bezier with the end points (0, 0) and (1, 1): solve x, then return y.
fn bezier(x1: f32, y1: f32, x2: f32, y2: f32, x: f32) -> f32 {
    let bx = |u: f32| 3.0 * (1.0 - u).powi(2) * u * x1 + 3.0 * (1.0 - u) * u * u * x2 + u * u * u;
    let by = |u: f32| 3.0 * (1.0 - u).powi(2) * u * y1 + 3.0 * (1.0 - u) * u * u * y2 + u * u * u;
    let (mut lo, mut hi) = (0.0_f32, 1.0_f32);
    for _ in 0..20 {
        let mid = 0.5 * (lo + hi);
        if bx(mid) < x {
            lo = mid;
        } else {
            hi = mid;
        }
    }
    by(0.5 * (lo + hi))
}

/// A duration in seconds and the curve that shapes it.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Timing {
    pub secs: f32,
    pub curve: Curve,
}

// ---- The timing table. DESIGN section 5.1, row numbers in each comment. ----

/// Battery ring fill. Row 1: 350 ms, decelerate.
pub const RING_FILL: Timing = Timing { secs: 0.350, curve: Curve::Decelerate(1.0) };
/// Battery ring empty. Row 2: 500 ms, decelerate.
pub const RING_EMPTY: Timing = Timing { secs: 0.500, curve: Curve::Decelerate(1.0) };
/// Ring connect and disconnect cross-fade. Row 3: 175 ms, linear.
pub const RING_FADE: Timing = Timing { secs: 0.175, curve: Curve::Linear };
/// Switch track cross-fade. Row 13: 160 ms, linear.
pub const SWITCH_TRACK: Timing = Timing { secs: 0.160, curve: Curve::Linear };
/// Switch thumb slide. Row 14: 155 ms. The curve is UNKNOWN: DESIGN says "platform switch easing"
/// and gives no formula. The capture `anim-low-latency.mp4` jumps from frame 1 to frame 2, so it
/// does not show the slide. The port uses linear until a measurement exists.
pub const SWITCH_SLIDE: Timing = Timing { secs: 0.155, curve: Curve::Linear };
/// Sheet and dialog enter. Row 17: 260 ms, overshoot 1.6.
pub const SHEET_ENTER: Timing = Timing { secs: 0.260, curve: Curve::Overshoot(1.6) };
/// Page enter, alpha. Row 25: 240 ms, decelerate cubic.
pub const PAGE_ENTER_FADE: Timing = Timing { secs: 0.240, curve: Curve::DecelerateCubic };
/// Page enter, vertical move. Row 25: 300 ms, decelerate cubic.
pub const PAGE_ENTER_MOVE: Timing = Timing { secs: 0.300, curve: Curve::DecelerateCubic };
/// The page below an opening page, alpha. Row 26: 220 ms, decelerate cubic.
pub const PAGE_BELOW_FADE: Timing = Timing { secs: 0.220, curve: Curve::DecelerateCubic };
/// The page below an opening page, scale. Row 26: 260 ms, decelerate cubic.
pub const PAGE_BELOW_SCALE: Timing = Timing { secs: 0.260, curve: Curve::DecelerateCubic };
/// A page that comes back, alpha. Row 27: 220 ms, decelerate cubic.
pub const PAGE_RETURN_FADE: Timing = Timing { secs: 0.220, curve: Curve::DecelerateCubic };
/// A page that comes back, scale. Row 27: 260 ms, decelerate cubic.
pub const PAGE_RETURN_SCALE: Timing = Timing { secs: 0.260, curve: Curve::DecelerateCubic };
/// A page that closes, alpha. Row 28: 180 ms, accelerate cubic.
pub const PAGE_LEAVE_FADE: Timing = Timing { secs: 0.180, curve: Curve::AccelerateCubic };
/// A page that closes, vertical move. Row 28: 200 ms, accelerate cubic.
pub const PAGE_LEAVE_MOVE: Timing = Timing { secs: 0.200, curve: Curve::AccelerateCubic };

static REDUCED: AtomicBool = AtomicBool::new(false);

/// Turn every animation into an instant state change. The default is off. egui 0.37 does not expose
/// the OS reduced-motion preference, so `main.rs` sets this value.
pub fn set_reduced_motion(on: bool) {
    REDUCED.store(on, Ordering::Relaxed);
}

/// True when every animation is an instant state change.
pub fn reduced_motion() -> bool {
    REDUCED.load(Ordering::Relaxed)
}

/// The state of one target-based animation.
#[derive(Clone, Copy)]
struct Tween {
    from: f32,
    to: f32,
    start: f64,
    timing: Timing,
}

/// The state of one one-shot animation.
#[derive(Clone, Copy)]
struct Run {
    start: f64,
    frame: u64,
}

fn now(ctx: &Context) -> f64 {
    ctx.input(|i| i.time)
}

/// The shown value of `st` at `now`, and true while the animation runs.
fn sample(st: &Tween, now: f64) -> (f32, bool) {
    let k = ((now - st.start) as f32 / st.timing.secs).clamp(0.0, 1.0);
    (st.from + (st.to - st.from) * st.timing.curve.at(k), k < 1.0)
}

/// A value that follows `target`. The first call snaps. A later change slides with `up` or `down`.
pub fn tween(ctx: &Context, key: Id, target: f32, up: Timing, down: Timing) -> f32 {
    let t0 = now(ctx);
    if reduced_motion() {
        ctx.data_mut(|d| d.insert_temp(key, Tween { from: target, to: target, start: t0, timing: up }));
        return target;
    }
    let Some(st) = ctx.data(|d| d.get_temp::<Tween>(key)) else {
        ctx.data_mut(|d| d.insert_temp(key, Tween { from: target, to: target, start: t0, timing: up }));
        return target;
    };
    if st.to == target {
        let (v, running) = sample(&st, t0);
        if running {
            ctx.request_repaint();
        }
        return v;
    }
    let (v, _) = sample(&st, t0);
    let timing = if target > v { up } else { down };
    ctx.data_mut(|d| d.insert_temp(key, Tween { from: v, to: target, start: t0, timing }));
    ctx.request_repaint();
    v
}

/// A value in 0..1 that follows a boolean with one timing for both directions.
pub fn fade(ctx: &Context, key: Id, on: bool, timing: Timing) -> f32 {
    tween(ctx, key, if on { 1.0 } else { 0.0 }, timing, timing)
}

/// The eased progress of a one-shot animation, 0..1. It starts on the first call and again after a
/// gap of two frames. An overshoot curve can return a value above 1.0.
pub fn run(ctx: &Context, key: Id, timing: Timing) -> f32 {
    if reduced_motion() {
        return 1.0;
    }
    let t0 = now(ctx);
    let frame = ctx.cumulative_frame_nr();
    let start = match ctx.data(|d| d.get_temp::<Run>(key)) {
        Some(r) if r.frame + 2 >= frame => r.start,
        _ => t0,
    };
    ctx.data_mut(|d| d.insert_temp(key, Run { start, frame }));
    let k = ((t0 - start) as f32 / timing.secs).clamp(0.0, 1.0);
    if k < 1.0 {
        ctx.request_repaint();
    }
    timing.curve.at(k)
}

/// The alpha and the scale of a sheet or a dialog.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Sheet {
    pub alpha: f32,
    pub scale: f32,
}

/// Sheet and dialog enter: alpha 0 to 1, scale 0.92 to 1 (row 17).
pub fn sheet_enter(ctx: &Context, key: Id) -> Sheet {
    let t = run(ctx, key, SHEET_ENTER);
    Sheet { alpha: t.clamp(0.0, 1.0), scale: 0.92 + 0.08 * t }
}

/// The alpha, the vertical offset in points (positive is below the resting place) and the scale of a
/// page change.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Page {
    pub alpha: f32,
    pub offset: f32,
    pub scale: f32,
}

/// A page that opens: alpha 0 to 1 and Y +4% to 0 (row 25). `height` is the window height.
pub fn page_enter(ctx: &Context, key: Id, height: f32) -> Page {
    let a = run(ctx, key.with("fade"), PAGE_ENTER_FADE).clamp(0.0, 1.0);
    let m = run(ctx, key.with("move"), PAGE_ENTER_MOVE).clamp(0.0, 1.0);
    Page { alpha: a, offset: (1.0 - m) * height * 0.04, scale: 1.0 }
}

/// The page below an opening page: alpha 1 to 0.6, scale 1 to 0.97 (row 26).
pub fn page_below(ctx: &Context, key: Id) -> Page {
    let a = run(ctx, key.with("fade"), PAGE_BELOW_FADE).clamp(0.0, 1.0);
    let s = run(ctx, key.with("scale"), PAGE_BELOW_SCALE).clamp(0.0, 1.0);
    Page { alpha: 1.0 - 0.4 * a, offset: 0.0, scale: 1.0 - 0.03 * s }
}

/// A page that comes back: alpha 0.6 to 1, scale 0.97 to 1 (row 27).
pub fn page_return(ctx: &Context, key: Id) -> Page {
    let a = run(ctx, key.with("fade"), PAGE_RETURN_FADE).clamp(0.0, 1.0);
    let s = run(ctx, key.with("scale"), PAGE_RETURN_SCALE).clamp(0.0, 1.0);
    Page { alpha: 0.6 + 0.4 * a, offset: 0.0, scale: 0.97 + 0.03 * s }
}

/// A page that closes: alpha 1 to 0 and Y 0 to +3% (row 28). `height` is the window height.
pub fn page_leave(ctx: &Context, key: Id, height: f32) -> Page {
    let a = run(ctx, key.with("fade"), PAGE_LEAVE_FADE).clamp(0.0, 1.0);
    let m = run(ctx, key.with("move"), PAGE_LEAVE_MOVE).clamp(0.0, 1.0);
    Page { alpha: 1.0 - a, offset: m * height * 0.03, scale: 1.0 }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn near(a: f32, b: f32) -> bool {
        (a - b).abs() < 0.02
    }

    #[test]
    fn curves_hit_both_ends() {
        let all = [
            Curve::Linear,
            Curve::Decelerate(1.0),
            Curve::Decelerate(2.0),
            Curve::AccelerateCubic,
            Curve::DecelerateCubic,
            Curve::Overshoot(1.4),
            Curve::Overshoot(1.6),
            Curve::Overshoot(2.8),
            Curve::Overshoot(3.0),
        ];
        for c in all {
            assert!(near(c.at(0.0), 0.0), "{c:?} at 0");
            assert!(near(c.at(1.0), 1.0), "{c:?} at 1");
        }
    }

    #[test]
    fn linear_and_decelerate_values() {
        assert!(near(Curve::Linear.at(0.5), 0.5));
        assert!(near(Curve::Decelerate(1.0).at(0.5), 0.5));
        assert!(near(Curve::Decelerate(2.0).at(0.5), 0.75));
    }

    #[test]
    fn overshoot_passes_one_then_returns() {
        let c = Curve::Overshoot(1.6);
        assert!(c.at(0.6) > 1.0, "overshoot passes 1.0: {}", c.at(0.6));
        assert!(near(c.at(0.5), 1.075));
    }

    #[test]
    fn cubic_curves_match_the_design_formula() {
        // cubic-bezier(0.0, 0.0, 0.2, 1.0) is fast out: about 84% at half time.
        assert!(near(Curve::DecelerateCubic.at(0.5), 0.84));
        // cubic-bezier(0.4, 0.0, 1.0, 1.0) is slow out: about 32% at half time.
        assert!(near(Curve::AccelerateCubic.at(0.5), 0.325));
    }

    #[test]
    fn timing_table_matches_design_section_5() {
        assert_eq!(RING_FILL, Timing { secs: 0.350, curve: Curve::Decelerate(1.0) });
        assert_eq!(RING_EMPTY, Timing { secs: 0.500, curve: Curve::Decelerate(1.0) });
        assert_eq!(RING_FADE, Timing { secs: 0.175, curve: Curve::Linear });
        assert_eq!(SWITCH_TRACK, Timing { secs: 0.160, curve: Curve::Linear });
        assert_eq!(SWITCH_SLIDE.secs, 0.155);
        assert_eq!(SHEET_ENTER, Timing { secs: 0.260, curve: Curve::Overshoot(1.6) });
        assert_eq!(PAGE_ENTER_FADE.secs, 0.240);
        assert_eq!(PAGE_ENTER_MOVE.secs, 0.300);
        assert_eq!(PAGE_BELOW_FADE.secs, 0.220);
        assert_eq!(PAGE_BELOW_SCALE.secs, 0.260);
        assert_eq!(PAGE_RETURN_FADE.secs, 0.220);
        assert_eq!(PAGE_RETURN_SCALE.secs, 0.260);
        assert_eq!(PAGE_LEAVE_FADE, Timing { secs: 0.180, curve: Curve::AccelerateCubic });
        assert_eq!(PAGE_LEAVE_MOVE, Timing { secs: 0.200, curve: Curve::AccelerateCubic });
    }
}
