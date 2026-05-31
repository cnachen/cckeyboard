use cortex_m::delay::Delay;
use stm32_hal2 as hal;
use usb_device::device::{
    StringDescriptors, UsbDevice, UsbDeviceBuilder, UsbVidPid,
};
use usbd_hid::descriptor::{KeyboardReport, SerializedDescriptor};
use usbd_hid::hid_class::{
    HIDClass, HidClassSettings, HidProtocol, HidSubClass, ProtocolModeConfig,
};

use hal::pac;
use hal::usb::{Usb1, Usb1BusType};

const USB_VID: u16 = 0x1209;
const USB_PID: u16 = 0x4110;
const USB_POLL_MS: u8 = 1;
const HID_RELEASE_MS: u32 = 12;

#[derive(Clone, Copy)]
pub struct KeyStroke {
    modifier: u8,
    keycode: u8,
}

impl KeyStroke {
    pub const fn new(keycode: u8) -> Self {
        Self {
            modifier: 0,
            keycode,
        }
    }

    pub const fn with_modifier(modifier: u8, keycode: u8) -> Self {
        Self { modifier, keycode }
    }
}

const MOD_LSHIFT: u8 = 0x02;

pub const KEY_LEFT: KeyStroke = KeyStroke::new(0x50);
pub const KEY_RIGHT: KeyStroke = KeyStroke::new(0x4f);
pub const KEY_UP: KeyStroke = KeyStroke::new(0x52);
pub const KEY_DOWN: KeyStroke = KeyStroke::new(0x51);
pub const KEY_ENTER: KeyStroke = KeyStroke::new(0x28);
pub const KEY_ESC: KeyStroke = KeyStroke::new(0x29);
pub const KEY_SPACE: KeyStroke = KeyStroke::new(0x2c);
pub const KEY_HOME: KeyStroke = KeyStroke::new(0x4a);
pub const KEY_END: KeyStroke = KeyStroke::new(0x4d);
pub const KEY_0: KeyStroke = KeyStroke::new(0x27);
pub const KEY_1: KeyStroke = KeyStroke::new(0x1e);
pub const KEY_2: KeyStroke = KeyStroke::new(0x1f);
pub const KEY_3: KeyStroke = KeyStroke::new(0x20);
pub const KEY_4: KeyStroke = KeyStroke::new(0x21);
pub const KEY_5: KeyStroke = KeyStroke::new(0x22);
pub const KEY_6: KeyStroke = KeyStroke::new(0x23);
pub const KEY_7: KeyStroke = KeyStroke::new(0x24);
pub const KEY_8: KeyStroke = KeyStroke::new(0x25);
pub const KEY_9: KeyStroke = KeyStroke::new(0x26);
pub const KEY_DOT: KeyStroke = KeyStroke::new(0x37);
pub const KEY_F7: KeyStroke = KeyStroke::new(0x40);
pub const KEY_F8: KeyStroke = KeyStroke::new(0x41);
pub const KEY_F9: KeyStroke = KeyStroke::new(0x42);
pub const KEY_F10: KeyStroke = KeyStroke::new(0x43);
pub const KEY_F11: KeyStroke = KeyStroke::new(0x44);
pub const KEY_F12: KeyStroke = KeyStroke::new(0x45);
pub const KEY_A: KeyStroke = KeyStroke::new(0x04);
pub const KEY_CAPITAL_A: KeyStroke =
    KeyStroke::with_modifier(MOD_LSHIFT, 0x04);
pub const KEY_I: KeyStroke = KeyStroke::new(0x0c);
pub const KEY_CAPITAL_I: KeyStroke =
    KeyStroke::with_modifier(MOD_LSHIFT, 0x0c);
pub const KEY_HASH: KeyStroke = KeyStroke::with_modifier(MOD_LSHIFT, 0x20);
pub const KEY_PERCENT: KeyStroke = KeyStroke::with_modifier(MOD_LSHIFT, 0x22);
pub const KEY_ASTERISK: KeyStroke = KeyStroke::with_modifier(MOD_LSHIFT, 0x25);

pub struct UsbKeyboard<'a> {
    usb_dev: UsbDevice<'a, Usb1BusType>,
    hid: HIDClass<'a, Usb1BusType>,
}

impl<'a> UsbKeyboard<'a> {
    pub fn new(
        usb_bus: &'a usb_device::bus::UsbBusAllocator<Usb1BusType>,
    ) -> Self {
        let hid = HIDClass::new_ep_in_with_settings(
            usb_bus,
            KeyboardReport::desc(),
            USB_POLL_MS,
            HidClassSettings {
                subclass: HidSubClass::Boot,
                protocol: HidProtocol::Keyboard,
                config: ProtocolModeConfig::ForceBoot,
                ..Default::default()
            },
        );

        let usb_dev =
            UsbDeviceBuilder::new(usb_bus, UsbVidPid(USB_VID, USB_PID))
                .strings(&[StringDescriptors::default()
                    .manufacturer("Cnachen")
                    .product("Claude Code Keyboard")
                    .serial_number("0001")])
                .unwrap()
                .device_class(0)
                .build();

        Self { usb_dev, hid }
    }

    pub fn new_bus(
        dp: pac::Peripherals,
        hclk: u32,
        ep_memory: &'static mut [u32],
    ) -> usb_device::bus::UsbBusAllocator<Usb1BusType> {
        Usb1BusType::new(
            Usb1::new(
                dp.OTG_FS_GLOBAL,
                dp.OTG_FS_DEVICE,
                dp.OTG_FS_PWRCLK,
                hclk,
            ),
            ep_memory,
        )
    }

    pub fn force_reset(&mut self, delay: &mut Delay) {
        delay.delay_ms(50);
        let _ = self.usb_dev.bus().force_reset(delay);
        delay.delay_ms(50);
    }

    pub fn poll(&mut self) {
        let _ = self.usb_dev.poll(&mut [&mut self.hid]);
    }

    pub fn send_keystroke(&mut self, key: KeyStroke, delay: &mut Delay) {
        if key.modifier != 0 {
            self.push_report(&key.modifier_report(), delay);
        }

        self.push_report(&key.to_report(), delay);
        self.push_report(&KeyboardReport::default(), delay);
    }

    fn push_report(&mut self, report: &KeyboardReport, delay: &mut Delay) {
        for _ in 0..64 {
            self.poll();
            if self.hid.push_input(report).is_ok() {
                break;
            }

            delay.delay_ms(1);
        }

        delay.delay_ms(HID_RELEASE_MS);
    }
}

impl KeyStroke {
    fn modifier_report(self) -> KeyboardReport {
        KeyboardReport {
            modifier: self.modifier,
            reserved: 0,
            leds: 0,
            keycodes: [0, 0, 0, 0, 0, 0],
        }
    }

    fn to_report(self) -> KeyboardReport {
        KeyboardReport {
            modifier: self.modifier,
            reserved: 0,
            leds: 0,
            keycodes: [self.keycode, 0, 0, 0, 0, 0],
        }
    }
}
