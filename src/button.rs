use stm32_hal2 as hal;

use hal::gpio::Pin;

const DEBOUNCE_MS: u32 = 20;
pub const LONG_PRESS_MS: u32 = 500;
const COMBO_WINDOW_MS: u32 = 50;

const KEY_COUNT: usize = 3;
const K1_BIT: u8 = 0b001;
const K2_BIT: u8 = 0b010;
const K3_BIT: u8 = 0b100;
const SELECTOR_CHORD_MASK: u8 = K1_BIT | K2_BIT | K3_BIT;

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum PhysicalKey {
    K1,
    K2,
    K3,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum KeyAction {
    None,
    Short(PhysicalKey),
    Long(PhysicalKey),
    Chord2(PhysicalKey, PhysicalKey),
    Chord3,
    SelectorEntry,
}

#[derive(Clone, Copy)]
struct KeyState {
    stable_pressed: bool,
    raw_pressed: bool,
    raw_changed_ms: u32,
    pressed_ms: u32,
    long_emitted: bool,
}

impl KeyState {
    const fn new() -> Self {
        Self {
            stable_pressed: false,
            raw_pressed: false,
            raw_changed_ms: 0,
            pressed_ms: 0,
            long_emitted: false,
        }
    }
}

pub struct KeyScanner {
    keys: [KeyState; KEY_COUNT],
    active_mask: u8,
    combo_mask: u8,
    combo_started_ms: u32,
    selector_emitted: bool,
}

impl KeyScanner {
    pub const fn new() -> Self {
        Self {
            keys: [KeyState::new(), KeyState::new(), KeyState::new()],
            active_mask: 0,
            combo_mask: 0,
            combo_started_ms: 0,
            selector_emitted: false,
        }
    }

    pub fn poll(&mut self, pins: [&Pin; KEY_COUNT], now_ms: u32) -> KeyAction {
        if let Some(action) = self.update_stable_states(pins, now_ms) {
            return action;
        }

        self.detect_long_press(now_ms)
    }

    fn update_stable_states(
        &mut self,
        pins: [&Pin; KEY_COUNT],
        now_ms: u32,
    ) -> Option<KeyAction> {
        for index in 0..KEY_COUNT {
            let raw_pressed = pins[index].is_low();
            if raw_pressed != self.keys[index].raw_pressed {
                self.keys[index].raw_pressed = raw_pressed;
                self.keys[index].raw_changed_ms = now_ms;
            }

            if self.keys[index].stable_pressed == self.keys[index].raw_pressed
            {
                continue;
            }

            if now_ms.saturating_sub(self.keys[index].raw_changed_ms)
                < DEBOUNCE_MS
            {
                continue;
            }

            self.keys[index].stable_pressed = self.keys[index].raw_pressed;
            if self.keys[index].stable_pressed {
                if let Some(action) = self.handle_press(index, now_ms) {
                    return Some(action);
                }
            } else if let Some(action) = self.handle_release(index) {
                return Some(action);
            }
        }

        None
    }

    fn handle_press(
        &mut self,
        index: usize,
        now_ms: u32,
    ) -> Option<KeyAction> {
        let bit = key_bit(index);
        self.keys[index].pressed_ms = now_ms;
        self.keys[index].long_emitted = false;
        self.active_mask |= bit;

        if self.active_mask == bit {
            self.combo_mask = bit;
            self.combo_started_ms = now_ms;
            self.selector_emitted = false;
            return None;
        }

        if now_ms.saturating_sub(self.combo_started_ms) <= COMBO_WINDOW_MS {
            self.combo_mask |= bit;
        }

        None
    }

    fn handle_release(&mut self, index: usize) -> Option<KeyAction> {
        let bit = key_bit(index);
        let stable_mask_before = self.active_mask;
        self.active_mask &= !bit;

        if self.is_combo_member(bit) {
            if self.active_mask & self.combo_mask == 0 {
                let combo_mask = self.combo_mask;
                self.combo_mask = if self.active_mask == 0 {
                    0
                } else {
                    self.active_mask
                };

                let action = if self.selector_emitted {
                    None
                } else {
                    chord_action(combo_mask)
                };

                if self.active_mask == 0 {
                    self.selector_emitted = false;
                }

                return action;
            }

            return None;
        }

        if stable_mask_before == bit {
            self.combo_mask = 0;
            self.selector_emitted = false;
        }

        if self.keys[index].long_emitted {
            return None;
        }

        Some(KeyAction::Short(key_from_index(index)))
    }

    fn detect_long_press(&mut self, now_ms: u32) -> KeyAction {
        if !self.selector_emitted
            && self.combo_mask == SELECTOR_CHORD_MASK
            && self.active_mask == SELECTOR_CHORD_MASK
            && now_ms.saturating_sub(self.combo_started_ms) >= LONG_PRESS_MS
        {
            self.selector_emitted = true;
            for key in &mut self.keys {
                key.long_emitted = true;
            }
            return KeyAction::SelectorEntry;
        }

        for index in 0..KEY_COUNT {
            let bit = key_bit(index);
            if !self.keys[index].stable_pressed
                || self.keys[index].long_emitted
            {
                continue;
            }

            if self.combo_mask & bit != 0 && self.combo_mask.count_ones() > 1 {
                continue;
            }

            if now_ms.saturating_sub(self.keys[index].pressed_ms)
                < LONG_PRESS_MS
            {
                continue;
            }

            self.keys[index].long_emitted = true;
            return KeyAction::Long(key_from_index(index));
        }

        KeyAction::None
    }

    fn is_combo_member(&self, bit: u8) -> bool {
        self.combo_mask.count_ones() > 1 && self.combo_mask & bit != 0
    }
}

fn key_bit(index: usize) -> u8 {
    1_u8 << index
}

fn key_from_index(index: usize) -> PhysicalKey {
    match index {
        0 => PhysicalKey::K1,
        1 => PhysicalKey::K2,
        _ => PhysicalKey::K3,
    }
}

fn chord_action(mask: u8) -> Option<KeyAction> {
    match mask {
        x if x == (K1_BIT | K2_BIT) => {
            Some(KeyAction::Chord2(PhysicalKey::K1, PhysicalKey::K2))
        }
        x if x == (K1_BIT | K3_BIT) => {
            Some(KeyAction::Chord2(PhysicalKey::K1, PhysicalKey::K3))
        }
        x if x == (K2_BIT | K3_BIT) => {
            Some(KeyAction::Chord2(PhysicalKey::K2, PhysicalKey::K3))
        }
        SELECTOR_CHORD_MASK => Some(KeyAction::Chord3),
        _ => None,
    }
}
