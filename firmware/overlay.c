// What a QMK + VIA firmware needs so the overlay can light the keys under
// your fingers and follow the layer. Paste into your keyboard's (or keymap's)
// .c file. Without it the overlay still draws layer 0, but says the firmware
// does not answer 0xA3.
//
// It rides VIA's raw HID: an id VIA does not use (VIA's run 0x01..0x15 and
// 0xFF) is answered here, every other id falls through to VIA untouched.
//
// Reply, 32 bytes, little-endian:
//   [0] OVERLAY_CMD   [1] OVERLAY_VER
//   [2..5] layer_state        [6..9] default_layer_state
//   [10..] one u16 per matrix row, bit = column: down now OR down at any
//          moment since the last ask, so a tap shorter than the poll still
//          shows for one frame
//   then one byte: encoder 0 since the last ask, bit 0 CCW, bit 1 CW
// Both latches clear once sent. Hence at most 10 rows of 16 columns.
// src-tauri/src/hid.rs reads this.
//
// If your keyboard already defines housekeeping_task_kb, process_record_kb or
// via_command_kb, merge these into yours.

#include QMK_KEYBOARD_H
#include <string.h>
#include "raw_hid.h"

#define OVERLAY_CMD 0xA3
#define OVERLAY_VER 1

_Static_assert(10 + 2 * MATRIX_ROWS + 1 <= 32, "overlay reply overflows");
_Static_assert(MATRIX_COLS <= 16, "overlay reply has 16 columns per row");

static matrix_row_t latched[MATRIX_ROWS];
static uint8_t      knob;

void housekeeping_task_kb(void) {
    for (uint8_t r = 0; r < MATRIX_ROWS; r++) {
        latched[r] |= matrix_get_row(r);
    }
    housekeeping_task_user();
}

bool process_record_kb(uint16_t keycode, keyrecord_t *record) {
#ifdef ENCODER_MAP_ENABLE
    // With ENCODER_MAP_ENABLE a turn arrives as a key event; col is the encoder.
    if (record->event.pressed && record->event.key.col == 0) {
        if (record->event.type == ENCODER_CCW_EVENT) knob |= 1;
        if (record->event.type == ENCODER_CW_EVENT) knob |= 2;
    }
#endif
    return process_record_user(keycode, record);
}

static void put32(uint8_t *p, uint32_t v) {
    for (uint8_t i = 0; i < 4; i++) p[i] = (uint8_t)(v >> (8 * i));
}

bool via_command_kb(uint8_t *data, uint8_t length) {
    if (data[0] != OVERLAY_CMD) {
        return false;
    }
    memset(data + 1, 0, length - 1);
    data[1] = OVERLAY_VER;
    put32(data + 2, layer_state);
    put32(data + 6, default_layer_state);
    for (uint8_t r = 0; r < MATRIX_ROWS; r++) {
        matrix_row_t row = latched[r] | matrix_get_row(r);
        data[10 + 2 * r] = (uint8_t)row;
        data[11 + 2 * r] = (uint8_t)(row >> 8);
        latched[r]       = 0;
    }
    data[10 + 2 * MATRIX_ROWS] = knob;
    knob                       = 0;
    raw_hid_send(data, length);
    return true;
}
