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
                    .product("Black Pill Morse Keyboard")
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

    pub fn send_ascii(&mut self, ch: char, delay: &mut Delay) {
        if let Some(report) = ascii_to_report(ch) {
            self.push_report(&report, delay);
            self.push_report(&KeyboardReport::default(), delay);
        }
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
