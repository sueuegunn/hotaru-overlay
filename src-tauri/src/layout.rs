//! Where the keys are: a VIA definition (the JSON you load into usevia.app's
//! Design tab), read at run time from whatever file the user registered.
//!
//! The parts used:
//! - `vendorId` / `productId` - which keyboard to ask over raw HID
//! - `matrix.rows` / `matrix.cols` - the size of the keymap
//! - `layouts.keymap` - the board as KLE rows. Each key's top-left legend is
//!   its matrix cell `"row,col"`. VIA's other legends that matter here:
//!   slot 8 `"group,choice"` marks a layout option (only choice 0 is drawn,
//!   which is what VIA shows until told otherwise), slot 9 `"e"` marks an
//!   encoder whose top-left legend is `"index,direction"`, and slot 9 `"eN"`
//!   is one key standing for both directions of encoder N.

use serde::Serialize;
use serde_json::{Map, Value};

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct Bounds {
    pub x: f64,
    pub y: f64,
    pub w: f64,
    pub h: f64,
}

/// Where a KLE key sits, in key units. `r` degrees clockwise about (`rx`, `ry`).
#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct Place {
    pub x: f64,
    pub y: f64,
    pub w: f64,
    pub h: f64,
    pub r: f64,
    pub rx: f64,
    pub ry: f64,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct LayoutKey {
    pub row: usize,
    pub col: usize,
    #[serde(flatten)]
    pub place: Place,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct Encoder {
    pub index: usize,
    /// 0 CCW, 1 CW; `None` is one key drawn with both halves.
    pub dir: Option<u8>,
    #[serde(flatten)]
    pub place: Place,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct Layout {
    pub name: String,
    pub vid: u16,
    pub pid: u16,
    pub rows: usize,
    pub cols: usize,
    pub bounds: Bounds,
    pub keys: Vec<LayoutKey>,
    pub encoders: Vec<Encoder>,
}

/// One key off the KLE walk: where it is, its twelve legend slots, and
/// whether KLE calls it a decal (drawn on the board, not a key).
struct Item {
    place: Place,
    legends: Vec<String>,
    decal: bool,
}

fn num(o: &Map<String, Value>, k: &str) -> Option<f64> {
    o.get(k).and_then(Value::as_f64)
}

/// KLE rows -> keys, following kle-serial: `x`/`y` are jumps, `w`/`h` apply to
/// the next key only, `r`/`rx`/`ry` persist, and setting `rx`/`ry` moves the
/// cursor to that point. A new row starts one unit down at `rx`.
fn unkle(rows: &[Value]) -> Result<Vec<Item>, String> {
    let mut items = Vec::new();
    let (mut r, mut rx, mut ry) = (0.0, 0.0, 0.0);
    let (mut x, mut y) = (0.0, 0.0);
    for row in rows {
        // A leading object outside any row is KLE's board metadata.
        let Some(row) = row.as_array() else { continue };
        let (mut w, mut h, mut decal) = (1.0, 1.0, false);
        for k in row {
            match k {
                Value::Object(o) => {
                    if let Some(v) = num(o, "r") {
                        r = v;
                    }
                    if let Some(v) = num(o, "rx") {
                        rx = v;
                        (x, y) = (rx, ry);
                    }
                    if let Some(v) = num(o, "ry") {
                        ry = v;
                        (x, y) = (rx, ry);
                    }
                    x += num(o, "x").unwrap_or(0.0);
                    y += num(o, "y").unwrap_or(0.0);
                    w = num(o, "w").unwrap_or(w);
                    h = num(o, "h").unwrap_or(h);
                    decal = o.get("d").and_then(Value::as_bool).unwrap_or(decal);
                }
                Value::String(legend) => {
                    items.push(Item {
                        place: Place { x, y, w, h, r, rx, ry },
                        legends: legend.split('\n').map(str::to_owned).collect(),
                        decal,
                    });
                    x += w;
                    (w, h, decal) = (1.0, 1.0, false);
                }
                other => return Err(format!("unexpected KLE entry {other}")),
            }
        }
        y += 1.0;
        x = rx;
    }
    Ok(items)
}

fn pair(s: &str) -> Option<(usize, usize)> {
    let (a, b) = s.split_once(',')?;
    Some((a.trim().parse().ok()?, b.trim().parse().ok()?))
}

fn hex_id(via: &Value, k: &str) -> Result<u16, String> {
    let s = via[k].as_str().ok_or(format!("no {k}"))?;
    let digits = s.trim().trim_start_matches("0x").trim_start_matches("0X");
    u16::from_str_radix(digits, 16).map_err(|_| format!("{k} {s:?} is not a 16-bit hex number"))
}

/// The four corners of a placed key, rotated.
fn corners(p: &Place) -> [(f64, f64); 4] {
    let (s, c) = p.r.to_radians().sin_cos();
    let turn = |x: f64, y: f64| {
        let (dx, dy) = (x - p.rx, y - p.ry);
        (p.rx + dx * c - dy * s, p.ry + dx * s + dy * c)
    };
    [
        turn(p.x, p.y),
        turn(p.x + p.w, p.y),
        turn(p.x, p.y + p.h),
        turn(p.x + p.w, p.y + p.h),
    ]
}

pub fn parse(text: &str) -> Result<Layout, String> {
    let via: Value = serde_json::from_str(text).map_err(|e| e.to_string())?;
    let dim = |k: &str| via["matrix"][k].as_u64().map(|n| n as usize);
    let (rows, cols) = dim("rows").zip(dim("cols")).ok_or("no matrix.rows / matrix.cols")?;
    let kle = via["layouts"]["keymap"].as_array().ok_or("no layouts.keymap")?;
    let (vid, pid) = (hex_id(&via, "vendorId")?, hex_id(&via, "productId")?);
    let name = via["name"].as_str().unwrap_or("").to_owned();

    let mut keys = Vec::new();
    let mut encoders = Vec::new();
    for item in unkle(kle)? {
        let slot = |i: usize| item.legends.get(i).map(String::as_str).unwrap_or("");
        if item.decal {
            continue;
        }
        if let Some((_, choice)) = pair(slot(8)) {
            if choice != 0 {
                continue;
            }
        }
        if let Some(enc) = slot(9).strip_prefix('e') {
            let (index, dir) = match (enc, pair(slot(0))) {
                ("", Some((index, dir))) if dir < 2 => (index, Some(dir as u8)),
                ("", _) => return Err(format!("encoder key {:?} needs \"index,direction\"", slot(0))),
                (n, _) => (n.parse().map_err(|_| format!("encoder legend \"e{n}\""))?, None),
            };
            encoders.push(Encoder { index, dir, place: item.place });
            continue;
        }
        let Some((row, col)) = pair(slot(0)).filter(|&(r, c)| r < rows && c < cols) else {
            return Err(format!("{:?} is not a cell of the {rows}x{cols} matrix", slot(0)));
        };
        keys.push(LayoutKey { row, col, place: item.place });
    }
    if keys.is_empty() {
        return Err("no keys".into());
    }

    let pts: Vec<(f64, f64)> =
        keys.iter().map(|k| &k.place).chain(encoders.iter().map(|e| &e.place)).flat_map(corners).collect();
    let min = |f: fn(&(f64, f64)) -> f64| pts.iter().map(f).fold(f64::INFINITY, f64::min);
    let max = |f: fn(&(f64, f64)) -> f64| pts.iter().map(f).fold(f64::NEG_INFINITY, f64::max);
    let (x0, y0) = (min(|p| p.0), min(|p| p.1));
    let bounds = Bounds { x: x0, y: y0, w: max(|p| p.0) - x0, h: max(|p| p.1) - y0 };
    Ok(Layout { name, vid, pid, rows, cols, bounds, keys, encoders })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn def(kle: &str) -> String {
        format!(
            r#"{{"name":"t","vendorId":"0x1234","productId":"0x0001","matrix":{{"rows":2,"cols":3}},"layouts":{{"keymap":{kle}}}}}"#
        )
    }

    #[test]
    fn widths_jumps_and_rows() {
        let l = parse(&def(r#"[["0,0",{"w":2},"0,1","0,2"],[{"x":0.5},"1,0"]]"#)).unwrap();
        assert_eq!((l.vid, l.pid, l.rows, l.cols), (0x1234, 1, 2, 3));
        let at: Vec<_> = l.keys.iter().map(|k| (k.row, k.col, k.place.x, k.place.y, k.place.w)).collect();
        assert_eq!(at, vec![(0, 0, 0.0, 0.0, 1.0), (0, 1, 1.0, 0.0, 2.0), (0, 2, 3.0, 0.0, 1.0), (1, 0, 0.5, 1.0, 1.0)]);
        assert_eq!(l.bounds, Bounds { x: 0.0, y: 0.0, w: 4.0, h: 2.0 });
    }

    #[test]
    fn rotation_moves_the_cursor_and_the_bounds() {
        let l = parse(&def(r#"[["0,0"],[{"r":90,"rx":3,"ry":0},"1,0","1,1"]]"#)).unwrap();
        let k = &l.keys[1].place;
        assert_eq!((k.x, k.y, k.r, k.rx, k.ry), (3.0, 0.0, 90.0, 3.0, 0.0));
        // Two keys turned a quarter clockwise about (3, 0) hang below it, to its left.
        assert!((l.bounds.h - 2.0).abs() < 1e-9 && (l.bounds.w - 3.0).abs() < 1e-9);
    }

    #[test]
    fn options_decals_and_metadata_are_skipped() {
        let l = parse(&def(r#"[{"name":"meta"},["0,0\n\n\n\n\n\n\n\n0,0","0,0\n\n\n\n\n\n\n\n0,1",{"d":true},"x"]]"#)).unwrap();
        assert_eq!(l.keys.len(), 1);
    }

    #[test]
    fn both_encoder_spellings() {
        let l = parse(&def(r#"[["0,0","0,0\n\n\n\n\n\n\n\n\ne","0,1\n\n\n\n\n\n\n\n\ne","\n\n\n\n\n\n\n\n\ne1"]]"#)).unwrap();
        let e: Vec<_> = l.encoders.iter().map(|e| (e.index, e.dir)).collect();
        assert_eq!(e, vec![(0, Some(0)), (0, Some(1)), (1, None)]);
    }

    #[test]
    fn a_legend_off_the_matrix_is_refused() {
        assert!(parse(&def(r#"[["9,9"]]"#)).unwrap_err().contains("9,9"));
    }

    #[test]
    fn missing_ids_are_refused() {
        let bad = def(r#"[["0,0"]]"#).replace(r#""vendorId":"0x1234","#, "");
        assert!(parse(&bad).unwrap_err().contains("vendorId"));
    }
}
