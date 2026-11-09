//! Where the notch sits on its monitor, and the hit-test maths that follows from that.
//!
//! Upstream Nectar hard-coded the top edge. Ledge adds a left-edge position, selected by the
//! `nectar-notch-position` setting (`top` or `left`, default `top`). Everything here is pure so
//! the geometry is unit tested; callers pass physical pixels.

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NotchPosition {
    Top,
    Left,
}

impl NotchPosition {
    /// Unknown or missing values fall back to `Top`, so upstream behaviour is the default.
    pub fn parse(value: Option<&str>) -> Self {
        match value {
            Some("left") => NotchPosition::Left,
            _ => NotchPosition::Top,
        }
    }
}

pub const SETTING_KEY: &str = "nectar-notch-position";

/// How far the notch window reaches into the screen, in logical px before nectar-scale and DPI.
/// Top keeps upstream's 420. Left panels grow sideways, and the widest (Control Center with many
/// widgets plus the dual clock) passes 780, so the left strip is deeper.
pub fn strip_depth_logical(pos: NotchPosition) -> f64 {
    match pos {
        NotchPosition::Top => 420.0,
        NotchPosition::Left => 960.0,
    }
}

pub fn notch_position(app: &tauri::AppHandle) -> NotchPosition {
    NotchPosition::parse(crate::utils::get_setting_str(app, SETTING_KEY).as_deref())
}

/// Axis-aligned rectangle in physical pixels. `contains` is inclusive on all sides, matching
/// the comparisons the mouse hook used before this module existed.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Rect {
    pub x: i32,
    pub y: i32,
    pub w: i32,
    pub h: i32,
}

impl Rect {
    pub fn new(x: i32, y: i32, w: i32, h: i32) -> Self {
        Rect { x, y, w, h }
    }

    pub fn contains(&self, px: i32, py: i32) -> bool {
        px >= self.x && px <= self.x + self.w && py >= self.y && py <= self.y + self.h
    }
}

/// The transparent strip the notch window occupies. `depth` is how far it reaches into the
/// screen from its edge: the window height for `Top`, the window width for `Left`.
pub fn window_rect(pos: NotchPosition, monitor: Rect, depth: i32) -> Rect {
    match pos {
        NotchPosition::Top => Rect::new(monitor.x, monitor.y, monitor.w, depth),
        NotchPosition::Left => Rect::new(monitor.x, monitor.y, depth, monitor.h),
    }
}

/// True when the cursor is in the thin reveal band along the notch's screen edge.
pub fn in_edge_band(pos: NotchPosition, monitor: Rect, band: i32, x: i32, y: i32) -> bool {
    match pos {
        NotchPosition::Top => y <= monitor.y + band && x >= monitor.x && x <= monitor.x + monitor.w,
        NotchPosition::Left => x <= monitor.x + band && y >= monitor.y && y <= monitor.y + monitor.h,
    }
}

/// Padding around the island that still counts as "on the notch".
#[derive(Clone, Copy, Debug)]
pub struct Pads {
    /// Along the edge, on both sides of the island.
    pub side: i32,
    /// Away from the edge, past the island's far side.
    pub far: i32,
    /// Extra margin on every side once the island already holds input.
    pub hyst: i32,
}

/// The box in which the notch window takes clicks. `island` is the island's absolute rect in
/// physical pixels; the box always reaches back to the window's screen edge.
pub fn interactive_box(pos: NotchPosition, window_origin: (i32, i32), island: Rect, pads: Pads) -> Rect {
    let (wx, wy) = window_origin;
    match pos {
        NotchPosition::Top => Rect::new(
            island.x - pads.side - pads.hyst,
            wy,
            island.w + 2 * pads.side + 2 * pads.hyst,
            island.h + pads.far + pads.hyst,
        ),
        NotchPosition::Left => Rect::new(
            wx,
            island.y - pads.side - pads.hyst,
            island.w + pads.far + pads.hyst,
            island.h + 2 * pads.side + 2 * pads.hyst,
        ),
    }
}

/// True when the cursor lies within `pad` of the box measured along the notch's edge. Used
/// with `in_edge_band` so approaching the island along the edge does not flicker.
pub fn near_edge_span(pos: NotchPosition, bx: Rect, pad: i32, x: i32, y: i32) -> bool {
    match pos {
        NotchPosition::Top => x >= bx.x - pad && x <= bx.x + bx.w + pad,
        NotchPosition::Left => y >= bx.y - pad && y <= bx.y + bx.h + pad,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const MON: Rect = Rect { x: 0, y: 0, w: 1920, h: 1080 };
    const PADS: Pads = Pads { side: 20, far: 5, hyst: 10 };

    #[test]
    fn parse_defaults_to_top() {
        assert_eq!(NotchPosition::parse(None), NotchPosition::Top);
        assert_eq!(NotchPosition::parse(Some("top")), NotchPosition::Top);
        assert_eq!(NotchPosition::parse(Some("bogus")), NotchPosition::Top);
        assert_eq!(NotchPosition::parse(Some("left")), NotchPosition::Left);
    }

    #[test]
    fn strip_depth_keeps_top_at_upstream_value() {
        assert_eq!(strip_depth_logical(NotchPosition::Top), 420.0);
        assert!(strip_depth_logical(NotchPosition::Left) >= 900.0);
    }

    #[test]
    fn top_window_matches_upstream() {
        assert_eq!(window_rect(NotchPosition::Top, MON, 420), Rect::new(0, 0, 1920, 420));
    }

    #[test]
    fn left_window_spans_full_height() {
        let mon = Rect::new(-1920, 100, 1920, 1080);
        assert_eq!(window_rect(NotchPosition::Left, mon, 420), Rect::new(-1920, 100, 420, 1080));
    }

    #[test]
    fn top_band() {
        assert!(in_edge_band(NotchPosition::Top, MON, 8, 900, 0));
        assert!(in_edge_band(NotchPosition::Top, MON, 8, 900, 8));
        assert!(!in_edge_band(NotchPosition::Top, MON, 8, 900, 9));
        assert!(!in_edge_band(NotchPosition::Top, MON, 8, 0, 500));
    }

    #[test]
    fn left_band() {
        assert!(in_edge_band(NotchPosition::Left, MON, 8, 0, 540));
        assert!(in_edge_band(NotchPosition::Left, MON, 8, 8, 540));
        assert!(!in_edge_band(NotchPosition::Left, MON, 8, 9, 540));
        assert!(!in_edge_band(NotchPosition::Left, MON, 8, 900, 0));
        assert!(!in_edge_band(NotchPosition::Left, MON, 8, 0, 1081));
    }

    #[test]
    fn left_band_on_offset_monitor() {
        let mon = Rect::new(1920, 0, 2560, 1440);
        assert!(in_edge_band(NotchPosition::Left, mon, 8, 1925, 700));
        assert!(!in_edge_band(NotchPosition::Left, mon, 8, 1919, 700));
    }

    #[test]
    fn top_box_matches_upstream_formula() {
        // Upstream: rx = island.x - pad_x - hyst, rw = w + 2 pad_x + 2 hyst,
        // ry_top = win.y, ry_bottom = win.y + h + pad_y + hyst.
        let island = Rect::new(890, 0, 140, 36);
        let b = interactive_box(NotchPosition::Top, (0, 0), island, PADS);
        assert_eq!(b, Rect::new(860, 0, 200, 51));
    }

    #[test]
    fn left_box_reaches_back_to_edge() {
        let island = Rect::new(0, 470, 36, 140);
        let b = interactive_box(NotchPosition::Left, (0, 0), island, PADS);
        assert_eq!(b, Rect::new(0, 440, 51, 200));
        assert!(b.contains(0, 540));
        assert!(b.contains(50, 540));
        assert!(!b.contains(52, 540));
        assert!(!b.contains(10, 439));
    }

    #[test]
    fn edge_span_runs_along_the_edge() {
        let b = Rect::new(0, 440, 51, 200);
        assert!(near_edge_span(NotchPosition::Left, b, 60, 0, 380));
        assert!(!near_edge_span(NotchPosition::Left, b, 60, 0, 379));
        let t = Rect::new(860, 0, 200, 51);
        assert!(near_edge_span(NotchPosition::Top, t, 60, 800, 0));
        assert!(!near_edge_span(NotchPosition::Top, t, 60, 799, 0));
    }
}
