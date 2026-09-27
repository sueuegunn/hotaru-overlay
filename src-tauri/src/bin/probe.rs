//! `cargo run --bin probe -- <VIA definition .json>`: the keyboard's answer to
//! 0xA3, printed, no window.
//!
//! For checking a freshly flashed firmware before blaming the overlay: press a
//! key and its cell shows as `#`, hold MO(1) and `layers` gains bit 1, turn the
//! knob and `knob` flashes 1 (CCW) or 2 (CW). Ctrl+C to stop.

use hidapi::HidApi;
use hotaru_overlay_lib::{hid, layout};

fn main() {
    let Some(path) = std::env::args().nth(1) else {
        eprintln!("usage: probe <VIA definition .json>");
        std::process::exit(2);
    };
    let layout = std::fs::read_to_string(&path).map_err(|e| e.to_string()).and_then(|t| layout::parse(&t));
    let layout = layout.unwrap_or_else(|e| {
        eprintln!("{path}: {e}");
        std::process::exit(2);
    });
    let mut api = HidApi::new().expect("hidapi");
    let Some(dev) = hid::open(&mut api, layout.vid, layout.pid) else {
        eprintln!("no raw HID interface for {:04x}:{:04x} (usage page ff60)", layout.vid, layout.pid);
        std::process::exit(1);
    };
    println!("connected; asking every {:?}", hid::PERIOD);

    let mut last = None;
    loop {
        match hid::ask(&dev, layout.rows) {
            Ok(Some(hid::Answer::Reply(r))) => {
                if last.as_ref() != Some(&r) {
                    let cells: Vec<String> = r
                        .rows
                        .iter()
                        .map(|row| (0..layout.cols).map(|c| if row >> c & 1 == 1 { '#' } else { '.' }).collect())
                        .collect();
                    println!(
                        "layers {:05b}  default {:05b}  knob {}  {}",
                        r.layer_state,
                        r.default_layer_state,
                        r.knob,
                        cells.join(" ")
                    );
                    last = Some(r);
                }
            }
            Ok(Some(hid::Answer::Old(v))) => {
                eprintln!("the firmware does not answer 0xA3 version {} (got {v}); flash a new build", hid::VER);
                std::process::exit(1);
            }
            Ok(_) => {}
            Err(e) => {
                eprintln!("lost the keyboard: {e}");
                std::process::exit(1);
            }
        }
        std::thread::sleep(hid::PERIOD);
    }
}
