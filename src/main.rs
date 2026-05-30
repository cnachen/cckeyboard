#![no_std]
#![no_main]

use cortex_m::delay::Delay;
use cortex_m::singleton;
use cortex_m_rt::entry;
use panic_halt as _;
use stm32_hal2 as hal;
use usb_device::device::{
    StringDescriptors, UsbDevice, UsbDeviceBuilder, UsbVidPid,
};
use usbd_hid::descriptor::{KeyboardReport, SerializedDescriptor};
use usbd_hid::hid_class::{
    HIDClass, HidClassSettings, HidProtocol, HidSubClass, ProtocolModeConfig,
};

use hal::clocks::{
    ApbPrescaler, Clocks, HclkPrescaler, InputSrc, PllSrc, Pllp, Pllq,
};
use hal::gpio::{OutputSpeed, OutputType, Pin, PinMode, Port, Pull};
use hal::pac;
use hal::usb::{Usb1, Usb1BusType};

const USB_VID: u16 = 0x1209;
const USB_PID: u16 = 0x4110;
const USB_POLL_MS: u32 = 1;
const HID_RELEASE_MS: u32 = 12;

const MORSE_UNIT_MS: u32 = 200;
const DOT_DASH_THRESHOLD_MS: u32 = MORSE_UNIT_MS * 2;
const LETTER_GAP_MS: u32 = MORSE_UNIT_MS * 3;
const WORD_GAP_MS: u32 = MORSE_UNIT_MS * 7;
const DEBOUNCE_MS: u32 = 20;
const BUTTON_IDLE_SAMPLE_MS: u32 = 20;

const KEY_PORT: Port = Port::A;
const KEY_PIN: u8 = 0;
const LED_PORT: Port = Port::C;
const LED_PIN: u8 = 13;
const USB_DM_PIN: u8 = 11;
const USB_DP_PIN: u8 = 12;
const USB_AF: u8 = 10;

#[entry]
fn main() -> ! {
    let cp = cortex_m::Peripherals::take().unwrap();
    let dp = pac::Peripherals::take().unwrap();

    let clocks = usb_clock_config();
    clocks.setup().unwrap();

    let mut delay = Delay::new(cp.SYST, clocks.systick());

    let mut led = Pin::new(LED_PORT, LED_PIN, PinMode::Output);
    led.output_type(OutputType::PushPull);
    led.set_high();

    let mut key = Pin::new(KEY_PORT, KEY_PIN, PinMode::Input);
    key.pull(Pull::Up);

    let mut usb_dm = Pin::new(Port::A, USB_DM_PIN, PinMode::Alt(USB_AF));
    usb_dm.output_type(OutputType::PushPull);
    usb_dm.output_speed(OutputSpeed::VeryHigh);
    usb_dm.pull(Pull::Floating);

    let mut usb_dp = Pin::new(Port::A, USB_DP_PIN, PinMode::Alt(USB_AF));
    usb_dp.output_type(OutputType::PushPull);
    usb_dp.output_speed(OutputSpeed::VeryHigh);
    usb_dp.pull(Pull::Floating);

    let ep_memory = singleton!(: [u32; 1024] = [0; 1024]).unwrap();
    let usb_bus = Usb1BusType::new(
        Usb1::new(
            dp.OTG_FS_GLOBAL,
            dp.OTG_FS_DEVICE,
            dp.OTG_FS_PWRCLK,
            clocks.hclk(),
        ),
        ep_memory,
    );

    let hid = HIDClass::new_ep_in_with_settings(
        &usb_bus,
        KeyboardReport::desc(),
        USB_POLL_MS as u8,
        HidClassSettings {
            subclass: HidSubClass::Boot,
            protocol: HidProtocol::Keyboard,
            config: ProtocolModeConfig::ForceBoot,
            ..Default::default()
        },
    );
    let usb_dev = UsbDeviceBuilder::new(&usb_bus, UsbVidPid(USB_VID, USB_PID))
        .strings(&[StringDescriptors::default()
            .manufacturer("Cnachen")
            .product("Black Pill Morse Keyboard")
            .serial_number("0001")])
        .unwrap()
        .device_class(0)
        .build();
    let mut usb_dev = usb_dev;
    let mut hid = hid;

    // Force a clean disconnect/reconnect so the host re-enumerates after flashing.
    delay.delay_ms(50);
    let _ = usb_dev.bus().force_reset(&mut delay);
    delay.delay_ms(50);

    let idle_is_high = sample_button_idle_level(&key, &mut delay);

    let mut morse = MorseDecoder::new();
    let mut elapsed_ms = 0_u32;
    let mut key_down = false;
    let mut press_start_ms = 0_u32;
    let mut release_start_ms = 0_u32;
    let mut last_space_emitted = false;

    loop {
        poll_usb(&mut usb_dev, &mut hid);

        let pressed = is_button_pressed(&key, idle_is_high);
        if pressed != key_down {
            delay.delay_ms(DEBOUNCE_MS);
            poll_usb(&mut usb_dev, &mut hid);
            let stable_pressed = is_button_pressed(&key, idle_is_high);
            if stable_pressed != key_down {
                key_down = stable_pressed;
                if key_down {
                    press_start_ms = elapsed_ms;
                    led.set_low();
                } else {
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
            }
        } else if !key_down && morse.has_symbols() {
            let idle_ms = elapsed_ms.saturating_sub(release_start_ms);
            if idle_ms >= WORD_GAP_MS {
                if let Some(ch) = morse.take_char() {
                    send_ascii(ch, &mut usb_dev, &mut hid, &mut delay);
                }
                if !last_space_emitted {
                    send_ascii(' ', &mut usb_dev, &mut hid, &mut delay);
                    last_space_emitted = true;
                }
            } else if idle_ms >= LETTER_GAP_MS {
                if let Some(ch) = morse.take_char() {
                    send_ascii(ch, &mut usb_dev, &mut hid, &mut delay);
                    last_space_emitted = false;
                }
            }
        }

        delay.delay_ms(USB_POLL_MS);
        elapsed_ms = elapsed_ms.saturating_add(USB_POLL_MS);
    }
}

fn usb_clock_config() -> Clocks {
    Clocks {
        input_src: InputSrc::Pll(PllSrc::Hsi),
        pllm: 8,
        plln: 96,
        pllp: Pllp::Div2,
        pllq: Pllq::Div4,
        hclk_prescaler: HclkPrescaler::Div1,
        apb1_prescaler: ApbPrescaler::Div2,
        apb2_prescaler: ApbPrescaler::Div1,
        hse_bypass: false,
        security_system: false,
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

fn poll_usb<'a>(usb_dev: &mut UsbDevice<'a, Usb1BusType>, hid: &mut HIDClass<'a, Usb1BusType>) {
    let _ = usb_dev.poll(&mut [hid]);
}

fn send_ascii<'a>(
    ch: char,
    usb_dev: &mut UsbDevice<'a, Usb1BusType>,
    hid: &mut HIDClass<'a, Usb1BusType>,
    delay: &mut Delay,
) {
    if let Some(report) = ascii_to_report(ch) {
        push_report(&report, usb_dev, hid, delay);
        push_report(&KeyboardReport::default(), usb_dev, hid, delay);
    }
}

fn push_report<'a>(
    report: &KeyboardReport,
    usb_dev: &mut UsbDevice<'a, Usb1BusType>,
    hid: &mut HIDClass<'a, Usb1BusType>,
    delay: &mut Delay,
) {
    for _ in 0..64 {
        poll_usb(usb_dev, hid);
        if hid.push_input(report).is_ok() {
            break;
        }

        delay.delay_ms(1);
    }

    delay.delay_ms(HID_RELEASE_MS);
}

fn ascii_to_report(ch: char) -> Option<KeyboardReport> {
    let (modifier, keycode) = match ch {
        'a'..='z' => (0x00, 0x04 + (ch as u8 - b'a')),
        'A'..='Z' => (0x02, 0x04 + (ch as u8 - b'A')),
        '1'..='9' => (0x00, 0x1e + (ch as u8 - b'1')),
        '0' => (0x00, 0x27),
        ' ' => (0x00, 0x2c),
        _ => return None,
    };

    Some(KeyboardReport {
        modifier,
        reserved: 0,
        leds: 0,
        keycodes: [keycode, 0, 0, 0, 0, 0],
    })
}

#[derive(Clone, Copy)]
enum Symbol {
    Dot,
    Dash,
}

struct MorseDecoder {
    len: u8,
    bits: u8,
}

impl MorseDecoder {
    const fn new() -> Self {
        Self { len: 0, bits: 0 }
    }

    fn push(&mut self, symbol: Symbol) {
        if self.len >= 5 {
            self.clear();
            return;
        }

        self.bits <<= 1;
        if let Symbol::Dash = symbol {
            self.bits |= 1;
        }
        self.len += 1;
    }

    fn has_symbols(&self) -> bool {
        self.len != 0
    }

    fn take_char(&mut self) -> Option<char> {
        let decoded = decode_morse(self.len, self.bits);
        self.clear();
        decoded
    }

    fn clear(&mut self) {
        self.len = 0;
        self.bits = 0;
    }
}

fn decode_morse(len: u8, bits: u8) -> Option<char> {
    match (len, bits) {
        (1, 0b0) => Some('E'),
        (1, 0b1) => Some('T'),
        (2, 0b00) => Some('I'),
        (2, 0b01) => Some('A'),
        (2, 0b10) => Some('N'),
        (2, 0b11) => Some('M'),
        (3, 0b000) => Some('S'),
        (3, 0b001) => Some('U'),
        (3, 0b010) => Some('R'),
        (3, 0b011) => Some('W'),
        (3, 0b100) => Some('D'),
        (3, 0b101) => Some('K'),
        (3, 0b110) => Some('G'),
        (3, 0b111) => Some('O'),
        (4, 0b0000) => Some('H'),
        (4, 0b0001) => Some('V'),
        (4, 0b0010) => Some('F'),
        (4, 0b0100) => Some('L'),
        (4, 0b0110) => Some('P'),
        (4, 0b0111) => Some('J'),
        (4, 0b1000) => Some('B'),
        (4, 0b1001) => Some('X'),
        (4, 0b1010) => Some('C'),
        (4, 0b1011) => Some('Y'),
        (4, 0b1100) => Some('Z'),
        (4, 0b1101) => Some('Q'),
        (5, 0b00000) => Some('5'),
        (5, 0b00001) => Some('4'),
        (5, 0b00011) => Some('3'),
        (5, 0b00111) => Some('2'),
        (5, 0b01111) => Some('1'),
        (5, 0b11111) => Some('0'),
        (5, 0b11110) => Some('9'),
        (5, 0b11100) => Some('8'),
        (5, 0b11000) => Some('7'),
        (5, 0b10000) => Some('6'),
        _ => None,
    }
}
