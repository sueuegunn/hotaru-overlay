//! What the keys type: VIA's own save file (the `.layout.json` its Save
//! button writes), read at run time from whatever file the user registered.
//!
//! `layers[l][row * cols + col]` is VIA's flat order, and `encoders[e][l]` is
//! `[ccw, cw]`. `vendorProductId` (vid << 16 | pid) says which keyboard the
//! file came from, which is how a keymap for another board is caught.

use serde::{Deserialize, Serialize};

use crate::layout::Layout;

#[derive(Debug, Clone, Serialize)]
pub struct Keymap {
    /// `layers[l][row * cols + col]`
    pub layers: Vec<Vec<String>>,
    /// `encoders[e][l] = [ccw, cw]`
    pub encoders: Vec<Vec<[String; 2]>>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct ViaSave {
    #[serde(default)]
    vendor_product_id: Option<u32>,
    layers: Vec<Vec<String>>,
    #[serde(default)]
    encoders: Vec<Vec<[String; 2]>>,
}

pub fn parse(text: &str, layout: &Layout) -> Result<Keymap, String> {
    let save: ViaSave = serde_json::from_str(text).map_err(|e| e.to_string())?;
    if let Some(id) = save.vendor_product_id {
        let want = u32::from(layout.vid) << 16 | u32::from(layout.pid);
        if id != want {
            return Err(format!(
                "this keymap is for {:04x}:{:04x}, the layout is {:04x}:{:04x}",
                id >> 16,
                id & 0xffff,
                layout.vid,
                layout.pid
            ));
        }
    }
    if save.layers.is_empty() {
        return Err("no layers".into());
    }
    let cells = layout.rows * layout.cols;
    if let Some(bad) = save.layers.iter().position(|l| l.len() != cells) {
        return Err(format!("layer {bad} has {} cells, the matrix has {cells}", save.layers[bad].len()));
    }
    Ok(Keymap { layers: save.layers, encoders: save.encoders })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn layout() -> Layout {
        crate::layout::parse(
            r#"{"vendorId":"0x1234","productId":"0x0001","matrix":{"rows":1,"cols":2},"layouts":{"keymap":[["0,0","0,1"]]}}"#,
        )
        .unwrap()
    }

    #[test]
    fn a_via_save_reads() {
        let km = parse(
            r#"{"name":"t","vendorProductId":305397761,"macros":[],"layers":[["KC_A","KC_B"],["KC_TRNS","KC_1"]],"encoders":[[["KC_VOLD","KC_VOLU"],["KC_TRNS","KC_TRNS"]]]}"#,
            &layout(),
        )
        .unwrap();
        assert_eq!(km.layers.len(), 2);
        assert_eq!(km.encoders[0][0], ["KC_VOLD".to_string(), "KC_VOLU".to_string()]);
    }

    #[test]
    fn another_keyboards_keymap_is_refused() {
        let e = parse(r#"{"vendorProductId":1,"layers":[["KC_A","KC_B"]]}"#, &layout()).unwrap_err();
        assert!(e.contains("0000:0001"), "{e}");
    }

    #[test]
    fn a_layer_of_the_wrong_size_is_refused() {
        assert!(parse(r#"{"layers":[["KC_A"]]}"#, &layout()).unwrap_err().contains("the matrix has 2"));
    }
}
