//! Asking the keyboard what is going on.
//!
//! A firmware that wants the overlay to follow it answers raw HID command
//! 0xA3 (on VIA's raw HID interface; see firmware/overlay.c) with its layer
//! state and every matrix cell that is down or went down since the last ask.
//! That is the only way to light the key under a finger: MO() keys send the
//! host nothing, and a keycode seen by the OS cannot say which of two
//! identical keys it came from. A keyboard without it still gets its layer 0
//! drawn; the overlay just says the firmware does not answer.
//!
//! The 32-byte reply, little-endian:
//!   [0] CMD  [1] VER  [2..5] layer_state  [6..9] default_layer_state
//!   [10..] one u16 per matrix row (bit = column)  then one byte for encoder 0
//!   (bit 0 CCW, bit 1 CW since the last ask)
//! so it carries at most MAX_ROWS rows of MAX_COLS columns.

use std::time::Duration;

use std::sync::atomic::Ordering;

use hidapi::{HidApi, HidDevice};
use tauri::{AppHandle, Manager};

use crate::state::{self, AppState};

/// QMK's raw HID interface (RAW_USAGE_PAGE / RAW_USAGE_ID).
const USAGE_PAGE: u16 = 0xFF60;
const USAGE: u16 = 0x61;
const REPORT: usize = 32;

pub const CMD: u8 = 0xA3;
pub const VER: u8 = 1;
pub const MAX_ROWS: usize = (REPORT - 10 - 1) / 2;
pub const MAX_COLS: usize = 16;
/// What VIA answers to an id it does not know: a firmware without 0xA3.
const UNHANDLED: u8 = 0xFF;

/// 250 asks a second. A round trip is about 1ms, so this is mostly waiting;
/// at 16ms a press showed about 9ms late on average. The firmware latches in between, so this sets how late
/// a press shows, not whether it shows.
pub const PERIOD: Duration = Duration::from_millis(4);
const RETRY: Duration = Duration::from_secs(1);

#[derive(Debug, Clone, PartialEq)]
pub struct Reply {
    pub layer_state: u32,
    pub default_layer_state: u32,
    pub rows: Vec<u16>,
    pub knob: u8,
}

#[derive(Debug, PartialEq)]
pub enum Answer {
    Reply(Reply),
    /// The keyboard runs a firmware from before 0xA3, or another version.
    Old(u8),
    /// Some other packet (a VIA reply meant for another program).
    Other,
}

fn u32_at(b: &[u8], i: usize) -> u32 {
    u32::from_le_bytes([b[i], b[i + 1], b[i + 2], b[i + 3]])
}

pub fn parse(buf: &[u8], rows: usize) -> Answer {
    if buf.len() < REPORT {
        return Answer::Other;
    }
    match buf[0] {
        UNHANDLED => return Answer::Old(0),
        CMD => {}
        _ => return Answer::Other,
    }
    if buf[1] != VER {
        return Answer::Old(buf[1]);
    }
    let rows_at = 10;
    Answer::Reply(Reply {
        layer_state: u32_at(buf, 2),
        default_layer_state: u32_at(buf, 6),
        rows: (0..rows).map(|r| u16::from_le_bytes([buf[rows_at + 2 * r], buf[rows_at + 2 * r + 1]])).collect(),
        knob: buf[rows_at + 2 * rows],
    })
}

pub fn open(api: &mut HidApi, vid: u16, pid: u16) -> Option<HidDevice> {
    let _ = api.refresh_devices();
    api.device_list()
        .find(|d| d.vendor_id() == vid && d.product_id() == pid && d.usage_page() == USAGE_PAGE && d.usage() == USAGE)
        .and_then(|d| d.open_device(api).ok())
}

/// One ask. `Ok(None)` is a timeout; an `Err` means the device went away.
pub fn ask(dev: &HidDevice, rows: usize) -> hidapi::HidResult<Option<Answer>> {
    let mut buf = [0u8; REPORT];
    // Throw away whatever is queued first. A reply that missed its read
    // timeout still arrives, and without this every later ask reads the one
    // before it - each miss adds one ask of lag, for good, until the OS's input
    // buffer (32 reports on Windows) is full: about a second behind. Other programs' VIA replies land
    // here too and go the same way.
    while dev.read_timeout(&mut buf, 0)? > 0 {}
    // hidapi wants the report id first; QMK's raw HID has none, so 0.
    let mut out = [0u8; REPORT + 1];
    out[1] = CMD;
    dev.write(&out)?;
    // Other programs' VIA replies arrive here too (every open handle gets every
    // input report), so skip a few that are not ours.
    for _ in 0..4 {
        let n = dev.read_timeout(&mut buf, 100)?;
        if n == 0 {
            return Ok(None);
        }
        match parse(&buf[..n], rows) {
            Answer::Other => continue,
            a => return Ok(Some(a)),
        }
    }
    Ok(None)
}

/// The keyboard the registered layout names, and the generation it came from.
struct Target {
    vid: u16,
    pid: u16,
    rows: usize,
    fits: bool,
    generation: u64,
}

fn target(app: &AppHandle) -> Option<Target> {
    let state = app.state::<AppState>();
    let generation = state.generation.load(Ordering::SeqCst);
    let files = state.files.lock().unwrap();
    let l = files.layout.as_ref()?;
    Some(Target { vid: l.vid, pid: l.pid, rows: l.rows, fits: l.rows <= MAX_ROWS && l.cols <= MAX_COLS, generation })
}

/// Keep asking for as long as the app runs; reconnect when unplugged or when
/// a different layout is registered.
pub fn spawn(app: &AppHandle) {
    let app = app.clone();
    std::thread::spawn(move || {
        let mut api = match HidApi::new() {
            Ok(api) => api,
            Err(e) => {
                eprintln!("hidapi: {e}");
                return;
            }
        };
        loop {
            match target(&app) {
                Some(t) if !t.fits => state::set_board(&app, state::Link::TooLarge, None),
                Some(t) => {
                    if let Some(dev) = open(&mut api, t.vid, t.pid) {
                        let current = |app: &AppHandle| app.state::<AppState>().generation.load(Ordering::SeqCst);
                        while current(&app) == t.generation {
                            match ask(&dev, t.rows) {
                                Ok(Some(Answer::Reply(r))) => state::set_board(&app, state::Link::Up, Some(r)),
                                Ok(Some(Answer::Old(v))) => state::set_board(&app, state::Link::OldFirmware(v), None),
                                Ok(_) => {}
                                Err(_) => break,
                            }
                            std::thread::sleep(PERIOD);
                        }
                    }
                    state::set_board(&app, state::Link::Down, None);
                }
                None => state::set_board(&app, state::Link::Down, None),
            }
            std::thread::sleep(RETRY);
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    fn report() -> [u8; REPORT] {
        let mut b = [0u8; REPORT];
        b[0] = CMD;
        b[1] = VER;
        b[2] = 0b110; // layers 1 and 2
        b[6] = 1; // default layer 0
        b[10 + 2 * 3] = 0x01; // row 3, col 0
        b[10 + 2 * 3 + 1] = 0x01; // row 3, col 8
        b[10 + 2 * 6] = 2; // knob CW
        b
    }

    #[test]
    fn a_reply_decodes() {
        let Answer::Reply(r) = parse(&report(), 6) else { panic!() };
        assert_eq!(r.layer_state, 0b110);
        assert_eq!(r.default_layer_state, 1);
        assert_eq!(r.rows, vec![0, 0, 0, 0x0101, 0, 0]);
        assert_eq!(r.knob, 2);
    }

    #[test]
    fn the_limits_fit_the_report() {
        assert_eq!(MAX_ROWS, 10);
        assert!(10 + 2 * MAX_ROWS + 1 <= REPORT);
    }

    #[test]
    fn old_and_foreign_packets_are_told_apart() {
        let mut b = report();
        b[1] = VER + 1;
        assert_eq!(parse(&b, 6), Answer::Old(VER + 1));
        b[0] = UNHANDLED;
        assert_eq!(parse(&b, 6), Answer::Old(0));
        b[0] = 0x04; // a VIA get_keycode reply
        assert_eq!(parse(&b, 6), Answer::Other);
    }
}
