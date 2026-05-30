use cortex_m::delay::Delay;
use stm32_hal2 as hal;

use hal::gpio::Pin;

const DEBOUNCE_MS: u32 = 20;
const BUTTON_IDLE_SAMPLE_MS: u32 = 20;

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum ButtonEvent {
    None,
    Pressed,
    Released,
}

pub struct ButtonInput {
    idle_is_high: bool,
    is_down: bool,
}

impl ButtonInput {
    pub fn new(key: &Pin, delay: &mut Delay) -> Self {
        Self {
            idle_is_high: sample_button_idle_level(key, delay),
            is_down: false,
        }
    }

    pub fn poll(&mut self, key: &Pin, delay: &mut Delay) -> ButtonEvent {
        let pressed = is_button_pressed(key, self.idle_is_high);
        if pressed == self.is_down {
            return ButtonEvent::None;
        }

        delay.delay_ms(DEBOUNCE_MS);
        let stable_pressed = is_button_pressed(key, self.idle_is_high);
        if stable_pressed == self.is_down {
            return ButtonEvent::None;
        }

        self.is_down = stable_pressed;
        if self.is_down {
            ButtonEvent::Pressed
        } else {
            ButtonEvent::Released
        }
    }
}

fn sample_button_idle_level(key: &Pin, delay: &mut Delay) -> bool {
    let mut high_count = 0_u8;
    let mut low_count = 0_u8;

    for _ in 0..BUTTON_IDLE_SAMPLE_MS {
        if key.is_high() {
            high_count = high_count.saturating_add(1);
        } else {
            low_count = low_count.saturating_add(1);
        }
        delay.delay_ms(1);
    }

    high_count >= low_count
}

fn is_button_pressed(key: &Pin, idle_is_high: bool) -> bool {
    if idle_is_high {
        key.is_low()
    } else {
        key.is_high()
    }
}
