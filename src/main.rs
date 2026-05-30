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
mod morse;
mod usb_hid;

use button::{ButtonEvent, ButtonInput};
use morse::{
    MorseDecoder, Symbol, DOT_DASH_THRESHOLD_MS, LETTER_GAP_MS, WORD_GAP_MS,
};
use usb_hid::UsbKeyboard;

#[entry]
fn main() -> ! {
    let cp = cortex_m::Peripherals::take().unwrap();
    let dp = pac::Peripherals::take().unwrap();

    let clocks = board::usb_clock_config();
    clocks.setup().unwrap();

    let mut delay = Delay::new(cp.SYST, clocks.systick());

    let mut led = board::init_led();
    let key = board::init_key();
    board::init_usb_pins();

    let ep_memory = singleton!(: [u32; 1024] = [0; 1024]).unwrap();
    let usb_bus = UsbKeyboard::new_bus(dp, clocks.hclk(), ep_memory);
    let mut keyboard = UsbKeyboard::new(&usb_bus);
    keyboard.force_reset(&mut delay);

    let mut button = ButtonInput::new(&key, &mut delay);
    let mut morse = MorseDecoder::new();
    let mut elapsed_ms = 0_u32;
    let mut press_start_ms = 0_u32;
    let mut release_start_ms = 0_u32;
    let mut last_space_emitted = false;
    let mut key_down = false;

    loop {
        keyboard.poll();

        match button.poll(&key, &mut delay) {
            ButtonEvent::Pressed => {
                key_down = true;
                press_start_ms = elapsed_ms;
                led.set_low();
            }
            ButtonEvent::Released => {
                key_down = false;
                led.set_high();
                let pulse_ms = elapsed_ms.saturating_sub(press_start_ms);
                let symbol = if pulse_ms < DOT_DASH_THRESHOLD_MS {
                    Symbol::Dot
                } else {
                    Symbol::Dash
                };
                morse.push(symbol);
                release_start_ms = elapsed_ms;
                last_space_emitted = false;
            }
            ButtonEvent::None => {}
        }

        if !key_down && morse.has_symbols() {
            let idle_ms = elapsed_ms.saturating_sub(release_start_ms);
            if idle_ms >= WORD_GAP_MS {
                if let Some(ch) = morse.take_char() {
                    keyboard.send_ascii(ch, &mut delay);
                }
                if !last_space_emitted {
                    keyboard.send_ascii(' ', &mut delay);
                    last_space_emitted = true;
                }
            } else if idle_ms >= LETTER_GAP_MS {
                if let Some(ch) = morse.take_char() {
                    keyboard.send_ascii(ch, &mut delay);
                    last_space_emitted = false;
                }
            }
        }

        delay.delay_ms(1);
        elapsed_ms = elapsed_ms.saturating_add(1);
    }
}
