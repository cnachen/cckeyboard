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
    KeyStroke, MediaStroke, UsbKeyboard, KEY_0, KEY_1, KEY_2, KEY_3, KEY_4,
    KEY_5, KEY_6, KEY_7, KEY_8, KEY_9, KEY_A, KEY_ASTERISK, KEY_CAPITAL_A,
    KEY_CAPITAL_I, KEY_DOT, KEY_DOWN, KEY_END, KEY_ENTER, KEY_ESC, KEY_HASH,
    KEY_HOME, KEY_I, KEY_LEFT, KEY_PERCENT, KEY_RIGHT, KEY_SPACE, KEY_UP,
    MEDIA_PLAY_PAUSE, MEDIA_TRACK_NEXT, MEDIA_TRACK_PREVIOUS,
    MEDIA_VOLUME_DOWN, MEDIA_VOLUME_MUTE, MEDIA_VOLUME_UP,
};

enum Stroke {
    Key(KeyStroke),
    Media(MediaStroke),
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Mode {
    Vim,
    Number,
    Navigation,
    Media,
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

        let mode = if self.matches(&[SelectorToken::K1]) {
            Mode::Number
        } else if self.matches(&[SelectorToken::K3]) {
            Mode::Navigation
        } else if self.matches(&[SelectorToken::K1, SelectorToken::K3]) {
            Mode::Media
        } else {
            Mode::Vim
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
    let mut mode = Mode::Vim;
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
        match stroke {
            Stroke::Key(key) => keyboard.send_keystroke(key, delay),
            Stroke::Media(key) => keyboard.send_media_stroke(key, delay),
        }
    }
}

fn map_action(mode: Mode, action: KeyAction) -> Option<Stroke> {
    match mode {
        Mode::Vim => map_vim_mode(action),
        Mode::Number => map_number_mode(action),
        Mode::Navigation => map_navigation_mode(action),
        Mode::Media => map_media_mode(action),
    }
}

fn map_vim_mode(action: KeyAction) -> Option<Stroke> {
    match action {
        KeyAction::Short(PhysicalKey::K1) => Some(Stroke::Key(KEY_I)),
        KeyAction::Long(PhysicalKey::K1) => Some(Stroke::Key(KEY_CAPITAL_I)),
        KeyAction::Short(PhysicalKey::K2) => Some(Stroke::Key(KEY_ESC)),
        KeyAction::Long(PhysicalKey::K2) => Some(Stroke::Key(KEY_DOT)),
        KeyAction::Short(PhysicalKey::K3) => Some(Stroke::Key(KEY_A)),
        KeyAction::Long(PhysicalKey::K3) => Some(Stroke::Key(KEY_CAPITAL_A)),
        KeyAction::Chord2(PhysicalKey::K1, PhysicalKey::K2) => {
            Some(Stroke::Key(KEY_HASH))
        }
        KeyAction::Chord2(PhysicalKey::K1, PhysicalKey::K3) => {
            Some(Stroke::Key(KEY_PERCENT))
        }
        KeyAction::Chord2(PhysicalKey::K2, PhysicalKey::K3) => {
            Some(Stroke::Key(KEY_ASTERISK))
        }
        _ => None,
    }
}

fn map_number_mode(action: KeyAction) -> Option<Stroke> {
    match action {
        KeyAction::Short(PhysicalKey::K1) => Some(Stroke::Key(KEY_1)),
        KeyAction::Long(PhysicalKey::K1) => Some(Stroke::Key(KEY_4)),
        KeyAction::Short(PhysicalKey::K2) => Some(Stroke::Key(KEY_2)),
        KeyAction::Long(PhysicalKey::K2) => Some(Stroke::Key(KEY_5)),
        KeyAction::Short(PhysicalKey::K3) => Some(Stroke::Key(KEY_3)),
        KeyAction::Long(PhysicalKey::K3) => Some(Stroke::Key(KEY_6)),
        KeyAction::Chord2(PhysicalKey::K1, PhysicalKey::K2) => {
            Some(Stroke::Key(KEY_7))
        }
        KeyAction::Chord2(PhysicalKey::K1, PhysicalKey::K3) => {
            Some(Stroke::Key(KEY_8))
        }
        KeyAction::Chord2(PhysicalKey::K2, PhysicalKey::K3) => {
            Some(Stroke::Key(KEY_9))
        }
        KeyAction::Chord3 => Some(Stroke::Key(KEY_0)),
        _ => None,
    }
}

fn map_navigation_mode(action: KeyAction) -> Option<Stroke> {
    match action {
        KeyAction::Short(PhysicalKey::K1) => Some(Stroke::Key(KEY_LEFT)),
        KeyAction::Long(PhysicalKey::K1) => Some(Stroke::Key(KEY_HOME)),
        KeyAction::Short(PhysicalKey::K2) => Some(Stroke::Key(KEY_ENTER)),
        KeyAction::Long(PhysicalKey::K2) => Some(Stroke::Key(KEY_ESC)),
        KeyAction::Short(PhysicalKey::K3) => Some(Stroke::Key(KEY_RIGHT)),
        KeyAction::Long(PhysicalKey::K3) => Some(Stroke::Key(KEY_END)),
        KeyAction::Chord2(PhysicalKey::K1, PhysicalKey::K2) => {
            Some(Stroke::Key(KEY_UP))
        }
        KeyAction::Chord2(PhysicalKey::K1, PhysicalKey::K3) => {
            Some(Stroke::Key(KEY_SPACE))
        }
        KeyAction::Chord2(PhysicalKey::K2, PhysicalKey::K3) => {
            Some(Stroke::Key(KEY_DOWN))
        }
        _ => None,
    }
}

fn map_media_mode(action: KeyAction) -> Option<Stroke> {
    match action {
        KeyAction::Short(PhysicalKey::K1) => {
            Some(Stroke::Media(MEDIA_TRACK_PREVIOUS))
        }
        KeyAction::Short(PhysicalKey::K2) => {
            Some(Stroke::Media(MEDIA_PLAY_PAUSE))
        }
        KeyAction::Short(PhysicalKey::K3) => {
            Some(Stroke::Media(MEDIA_TRACK_NEXT))
        }
        KeyAction::Chord2(PhysicalKey::K1, PhysicalKey::K2) => {
            Some(Stroke::Media(MEDIA_VOLUME_DOWN))
        }
        KeyAction::Chord2(PhysicalKey::K1, PhysicalKey::K3) => {
            Some(Stroke::Media(MEDIA_VOLUME_MUTE))
        }
        KeyAction::Chord2(PhysicalKey::K2, PhysicalKey::K3) => {
            Some(Stroke::Media(MEDIA_VOLUME_UP))
        }
        _ => None,
    }
}

fn indicate_mode(mode: Mode, led: &mut hal::gpio::Pin, delay: &mut Delay) {
    let blink_count = match mode {
        Mode::Vim => 1,
        Mode::Number | Mode::Navigation => 2,
        Mode::Media => 3,
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
