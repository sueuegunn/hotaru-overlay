//! Geometry and window behaviour for the overlay.
//!
//! The window is shown once at startup and then left alone - showing and
//! hiding a window on every toggle steals focus on Windows, so "off" is drawn
//! by the page itself (a fully transparent, click-through window is invisible).

use tauri::{AppHandle, LogicalSize, Manager, PhysicalPosition, PhysicalSize, WebviewWindow};

use crate::state::{AppState, Position};

/// The brief: about one ninth of the screen. One ninth of the area is one
/// third of each side. Where it sits is `Settings::position`.
const DIVISOR: u32 = 3;

/// Space left between the overlay and the edge of the work area when it sits
/// against one, in logical pixels.
const MARGIN: f64 = 16.0;

pub fn configure(app: &AppHandle) -> tauri::Result<()> {
    let Some(win) = app.get_webview_window("overlay") else {
        return Ok(());
    };

    // The overlay is never interactive, so the whole window can pass clicks
    // through - no per-region hit testing, no cursor polling.
    win.set_ignore_cursor_events(true)?;
    win.set_always_on_top(true)?;
    let _ = win.set_visible_on_all_workspaces(true);

    let position = app.state::<AppState>().settings.lock().unwrap().position;
    place(&win, position)?;
    win.show()?;
    Ok(())
}

pub fn place(win: &WebviewWindow, position: Position) -> tauri::Result<()> {
    let monitor = match win.primary_monitor()? {
        Some(m) => Some(m),
        None => win.current_monitor()?,
    };
    let Some(monitor) = monitor else {
        // No monitor to measure: fall back to something sane rather than
        // leaving the window at its default size.
        win.set_size(LogicalSize::new(640.0, 320.0))?;
        return Ok(());
    };

    // The size is a ninth of the whole screen, as it always was; where it goes
    // is measured on the work area, so the taskbar never covers a corner.
    let size = monitor.size();
    let w = size.width / DIVISOR;
    let h = size.height / DIVISOR;
    let area = monitor.work_area();
    let margin = (MARGIN * monitor.scale_factor()).round() as i32;
    let (x, y) = origin(
        (area.position.x, area.position.y),
        (area.size.width, area.size.height),
        (w, h),
        margin,
        position,
    );

    win.set_size(PhysicalSize::new(w, h))?;
    win.set_position(PhysicalPosition::new(x, y))?;
    Ok(())
}

/// Top-left corner of a `win`-sized window in `position` inside the area at
/// `at` of `size`. The edges keep `margin` clear; the middle row and column are
/// centred exactly, so `Center` is where the overlay always used to be.
fn origin(at: (i32, i32), size: (u32, u32), win: (u32, u32), margin: i32, position: Position) -> (i32, i32) {
    let (col, row) = position.cell();
    let along = |start: i32, span: u32, len: u32, cell: u8| -> i32 {
        let free = span as i32 - len as i32;
        let m = margin.min(free.max(0) / 2);
        start
            + match cell {
                0 => m,
                1 => free / 2,
                _ => free - m,
            }
    };
    (along(at.0, size.0, win.0, col), along(at.1, size.1, win.1, row))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn nine_places_inside_the_area() {
        // A 1920x1080 screen with a 40px taskbar at the bottom.
        let at = (0, 0);
        let area = (1920, 1040);
        let win = (640, 360);
        let o = |p| origin(at, area, win, 16, p);
        assert_eq!(o(Position::TopLeft), (16, 16));
        assert_eq!(o(Position::Top), (640, 16));
        assert_eq!(o(Position::Center), (640, 340));
        assert_eq!(o(Position::BottomRight), (1920 - 640 - 16, 1040 - 360 - 16));
        assert_eq!(o(Position::Left).0, 16);
        assert_eq!(o(Position::Bottom).1 + 360, 1040 - 16);
    }

    #[test]
    fn the_area_can_start_off_origin() {
        // A second monitor to the left, taskbar on top.
        let p = origin((-1920, 48), (1920, 1032), (640, 360), 16, Position::TopRight);
        assert_eq!(p, (-1920 + 1920 - 640 - 16, 48 + 16));
    }
}
