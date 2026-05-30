#![no_std]
#![no_main]

use cortex_m::delay::Delay;
use cortex_m::singleton;
use cortex_m_rt::entry;
use hal::pac;
use panic_halt as _;
use stm32_hal2 as hal;

mod board;
mod button;
mod usb_hid;

use button::{KeyAction, KeyScanner, PhysicalKey};
use usb_hid::{
    KeyStroke, UsbKeyboard, KEY_0, KEY_1, KEY_2, KEY_3, KEY_4, KEY_5, KEY_6,
    KEY_7, KEY_8, KEY_9, KEY_DOWN, KEY_END, KEY_ENTER, KEY_ESC, KEY_F10,
    KEY_F11, KEY_F12, KEY_F7, KEY_F8, KEY_F9, KEY_HOME, KEY_LEFT, KEY_RIGHT,
    KEY_SPACE, KEY_UP,
};

#[derive(Clone, Copy, PartialEq, Eq)]
enum Mode {
    Default,
    Media,
    Number,
}

#[derive(Clone, Copy)]
enum SelectorToken {
    K1,
    K3,
}

struct ModeSelector {
    active: bool,
    len: usize,
    tokens: [SelectorToken; 8],
}

impl ModeSelector {
    const fn new() -> Self {
        Self {
            active: false,
            len: 0,
            tokens: [SelectorToken::K1; 8],
        }
    }

    fn enter(&mut self) {
        self.active = true;
        self.len = 0;
    }

    fn push(&mut self, token: SelectorToken) {
        if self.len < self.tokens.len() {
            self.tokens[self.len] = token;
            self.len += 1;
        }
    }

    fn confirm(&mut self) -> Mode {
        self.active = false;

        let mode = if self.matches(&[
            SelectorToken::K1,
            SelectorToken::K3,
            SelectorToken::K1,
            SelectorToken::K3,
        ]) {
            Mode::Media
        } else if self.matches(&[
            SelectorToken::K1,
            SelectorToken::K1,
            SelectorToken::K3,
            SelectorToken::K3,
        ]) {
            Mode::Number
        } else {
            Mode::Default
        };

        self.len = 0;
        mode
    }

    fn matches(&self, expected: &[SelectorToken]) -> bool {
        if self.len != expected.len() {
            return false;
        }

        for (index, token) in expected.iter().enumerate() {
            if !selector_token_eq(self.tokens[index], *token) {
                return false;
            }
        }

        true
    }
}

#[entry]
fn main() -> ! {
    let cp = cortex_m::Peripherals::take().unwrap();
    let dp = pac::Peripherals::take().unwrap();

    let clocks = board::usb_clock_config();
    clocks.setup().unwrap();

    let mut delay = Delay::new(cp.SYST, clocks.systick());

    let mut led = board::init_led();
    let keys = board::init_keys();
    board::init_usb_pins();

    let ep_memory = singleton!(: [u32; 1024] = [0; 1024]).unwrap();
    let usb_bus = UsbKeyboard::new_bus(dp, clocks.hclk(), ep_memory);
    let mut keyboard = UsbKeyboard::new(&usb_bus);
    keyboard.force_reset(&mut delay);

    let mut scanner = KeyScanner::new();
    let mut selector = ModeSelector::new();
    let mut mode = Mode::Default;
    let mut elapsed_ms = 0_u32;

    loop {
        keyboard.poll();

        let action = scanner.poll([&keys.k1, &keys.k2, &keys.k3], elapsed_ms);
        handle_action(
            action,
            &mut selector,
            &mut mode,
            &mut keyboard,
            &mut led,
            &mut delay,
        );

        delay.delay_ms(1);
        elapsed_ms = elapsed_ms.saturating_add(1);
    }
}

fn handle_action(
    action: KeyAction,
    selector: &mut ModeSelector,
    mode: &mut Mode,
    keyboard: &mut UsbKeyboard,
    led: &mut hal::gpio::Pin,
    delay: &mut Delay,
) {
    if matches!(action, KeyAction::None) {
        return;
    }

    if matches!(action, KeyAction::SelectorEntry) {
        selector.enter();
        led.set_low();
        return;
    }

    if selector.active {
        match action {
            KeyAction::Short(PhysicalKey::K1) => {
                selector.push(SelectorToken::K1)
            }
            KeyAction::Short(PhysicalKey::K3) => {
                selector.push(SelectorToken::K3)
            }
            KeyAction::Short(PhysicalKey::K2) => {
                *mode = selector.confirm();
                indicate_mode(*mode, led, delay);
            }
            _ => {}
        }
        return;
    }

    if let Some(stroke) = map_action(*mode, action) {
        keyboard.send_keystroke(stroke, delay);
    }
}

fn map_action(mode: Mode, action: KeyAction) -> Option<KeyStroke> {
    match mode {
        Mode::Default => map_default_mode(action),
        Mode::Media => map_media_mode(action),
        Mode::Number => map_number_mode(action),
    }
}

fn map_default_mode(action: KeyAction) -> Option<KeyStroke> {
    match action {
        KeyAction::Short(PhysicalKey::K1) => Some(KEY_LEFT),
        KeyAction::Long(PhysicalKey::K1) => Some(KEY_HOME),
        KeyAction::Short(PhysicalKey::K2) => Some(KEY_ENTER),
        KeyAction::Long(PhysicalKey::K2) => Some(KEY_ESC),
        KeyAction::Short(PhysicalKey::K3) => Some(KEY_RIGHT),
        KeyAction::Long(PhysicalKey::K3) => Some(KEY_END),
        KeyAction::Chord2(PhysicalKey::K1, PhysicalKey::K2) => Some(KEY_UP),
        KeyAction::Chord2(PhysicalKey::K1, PhysicalKey::K3) => Some(KEY_SPACE),
        KeyAction::Chord2(PhysicalKey::K2, PhysicalKey::K3) => Some(KEY_DOWN),
        _ => None,
    }
}

fn map_media_mode(action: KeyAction) -> Option<KeyStroke> {
    match action {
        KeyAction::Short(PhysicalKey::K1) => Some(KEY_F7),
        KeyAction::Long(PhysicalKey::K1) => Some(KEY_F10),
        KeyAction::Short(PhysicalKey::K2) => Some(KEY_F8),
        KeyAction::Long(PhysicalKey::K2) => Some(KEY_F11),
        KeyAction::Short(PhysicalKey::K3) => Some(KEY_F9),
        KeyAction::Long(PhysicalKey::K3) => Some(KEY_F12),
        KeyAction::Chord2(PhysicalKey::K1, PhysicalKey::K2) => Some(KEY_HOME),
        KeyAction::Chord2(PhysicalKey::K1, PhysicalKey::K3) => Some(KEY_SPACE),
        KeyAction::Chord2(PhysicalKey::K2, PhysicalKey::K3) => Some(KEY_END),
        _ => None,
    }
}

fn map_number_mode(action: KeyAction) -> Option<KeyStroke> {
    match action {
        KeyAction::Short(PhysicalKey::K1) => Some(KEY_1),
        KeyAction::Long(PhysicalKey::K1) => Some(KEY_4),
        KeyAction::Short(PhysicalKey::K2) => Some(KEY_2),
        KeyAction::Long(PhysicalKey::K2) => Some(KEY_5),
        KeyAction::Short(PhysicalKey::K3) => Some(KEY_3),
        KeyAction::Long(PhysicalKey::K3) => Some(KEY_6),
        KeyAction::Chord2(PhysicalKey::K1, PhysicalKey::K2) => Some(KEY_7),
        KeyAction::Chord2(PhysicalKey::K1, PhysicalKey::K3) => Some(KEY_8),
        KeyAction::Chord2(PhysicalKey::K2, PhysicalKey::K3) => Some(KEY_9),
        KeyAction::Chord3 => Some(KEY_0),
        _ => None,
    }
}

fn indicate_mode(mode: Mode, led: &mut hal::gpio::Pin, delay: &mut Delay) {
    let blink_count = match mode {
        Mode::Default => 1,
        Mode::Media => 2,
        Mode::Number => 3,
    };

    for _ in 0..blink_count {
        led.set_low();
        delay.delay_ms(100);
        led.set_high();
        delay.delay_ms(100);
    }
}

fn selector_token_eq(lhs: SelectorToken, rhs: SelectorToken) -> bool {
    matches!(
        (lhs, rhs),
        (SelectorToken::K1, SelectorToken::K1)
            | (SelectorToken::K3, SelectorToken::K3)
    )
}
