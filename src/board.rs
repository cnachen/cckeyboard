use stm32_hal2 as hal;

use hal::clocks::{
    ApbPrescaler, Clocks, HclkPrescaler, InputSrc, PllSrc, Pllp, Pllq,
};
use hal::gpio::{OutputSpeed, OutputType, Pin, PinMode, Port, Pull};

const KEY_PORT: Port = Port::A;
const KEY_PIN: u8 = 0;
const LED_PORT: Port = Port::C;
const LED_PIN: u8 = 13;
const USB_DM_PIN: u8 = 11;
const USB_DP_PIN: u8 = 12;
const USB_AF: u8 = 10;

pub fn usb_clock_config() -> Clocks {
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

pub fn init_led() -> Pin {
    let mut led = Pin::new(LED_PORT, LED_PIN, PinMode::Output);
    led.output_type(OutputType::PushPull);
    led.set_high();
    led
}

pub fn init_key() -> Pin {
    let mut key = Pin::new(KEY_PORT, KEY_PIN, PinMode::Input);
    key.pull(Pull::Up);
    key
}

pub fn init_usb_pins() {
    let mut usb_dm = Pin::new(Port::A, USB_DM_PIN, PinMode::Alt(USB_AF));
    usb_dm.output_type(OutputType::PushPull);
    usb_dm.output_speed(OutputSpeed::VeryHigh);
    usb_dm.pull(Pull::Floating);

    let mut usb_dp = Pin::new(Port::A, USB_DP_PIN, PinMode::Alt(USB_AF));
    usb_dp.output_type(OutputType::PushPull);
    usb_dp.output_speed(OutputSpeed::VeryHigh);
    usb_dp.pull(Pull::Floating);
}
