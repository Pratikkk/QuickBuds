//! What the Overview's Hi-Res and 3D audio switches write, from the buds' state and the model (§9). No UI here:
//! `earbuds` (Slint) and `desktop-egui` both use it.

use crate::session::{Cmd, Snapshot};
use crate::types::*;

/// Our `spatialSwitch` key (realme): Hi-Res and 3D audio are written alone, never together (§9).
pub(crate) fn exclusive(s: &Snapshot) -> bool { !s.model.is_some_and(|m| m["spatialToggle"] == 1) }

/// 3D audio as HeyMelody picks it (§9): buds whose bitmap has `0x012A` (or a hand-picked model with head
/// tracking) take a type through `0x0422`, the rest feature `1B`.
pub(crate) fn spatial_by_type(s: &Snapshot) -> bool {
    exclusive(s) && (s.caps.supports(CMD_QUERY_SPATIAL_TYPE) || s.manual && head_tracking(s))
}

/// The model's `spatialTypes` has `2`: Off / Fixed / Head tracking instead of a plain switch.
pub(crate) fn head_tracking(s: &Snapshot) -> bool {
    s.model.is_some_and(|m| m["spatialModes"].as_array().is_some_and(|t| t.iter().any(|v| *v == 2)))
}

/// The codec picker replaces the Hi-Res write on `highAudio` models with the `0x0123` read (§9).
pub(crate) fn codec_picker(s: &Snapshot) -> bool { s.model.is_some_and(|m| m["hiRes"] == 1) && s.caps.supports(CMD_QUERY_CODEC_LIST) }

pub(crate) fn spatial_on(s: &Snapshot) -> bool {
    if spatial_by_type(s) { s.spatial_type.unwrap_or(0) != 0 } else { s.features.contains(&(FEATURE_SPATIAL, 1)) }
}

/// 3D audio to `t` (0 off): Hi-Res goes off after it where the two exclude each other, as HeyMelody sends them.
pub(crate) fn spatial_cmds(s: &Snapshot, t: u8) -> Vec<Cmd> {
    let drop_hires = t != 0 && exclusive(s) && s.features.contains(&(FEATURE_HIRES, 1));
    let mut cmds = vec![];
    if spatial_by_type(s) {
        cmds.push(Cmd::SpatialType(t));
        if drop_hires { cmds.push(Cmd::Features(vec![(FEATURE_HIRES, false)])); }
    } else if drop_hires {
        cmds.push(Cmd::Features(vec![(FEATURE_SPATIAL, true), (FEATURE_HIRES, false)]));
    } else {
        cmds.push(Cmd::Features(vec![(FEATURE_SPATIAL, t != 0)]));
    }
    cmds
}

/// The writes for one switch. Hi-Res: the codec write on codec-picker models; else 3D audio goes off first
/// where the two exclude each other.
pub(crate) fn feature_cmds(s: &Snapshot, id: u8, on: bool) -> Vec<Cmd> {
    match id {
        FEATURE_SPATIAL => spatial_cmds(s, on as u8),
        FEATURE_HIRES if codec_picker(s) => s.codec.map(|c| Cmd::Codec(c, on)).into_iter().collect(),
        FEATURE_HIRES if on && exclusive(s) && spatial_on(s) => if spatial_by_type(s) {
            vec![Cmd::SpatialType(0), Cmd::Features(vec![(FEATURE_HIRES, true)])]
        } else {
            vec![Cmd::Features(vec![(FEATURE_SPATIAL, false), (FEATURE_HIRES, true)])]
        },
        _ => vec![Cmd::Features(vec![(id, on)])],
    }
}
