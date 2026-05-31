# cckeyboard

`cckeyboard` is a compact three-key USB HID keyboard firmware for the
STM32F411. It turns three physical buttons into a modal input device
with short press, long press, two-key chord, and three-key selector
gestures.

The firmware enumerates as a standard USB keyboard plus a media HID
device, so it works with common desktop and laptop operating systems
without custom drivers.

## Features

- Three-button input with debounce and gesture detection
- Four logical modes: Vim, Number, Navigation, and Media
- USB full-speed HID keyboard output
- USB HID consumer/media control output
- Release packaging for `.bin` and `.hex` firmware artifacts

## Hardware

This project targets an STM32F411 with USB FS OTG enabled and assumes
the following pin assignments:

- `PA1`: key `K1`
- `PA2`: key `K2`
- `PA3`: key `K3`
- `PC13`: status LED
- `PA11`: USB DM
- `PA12`: USB DP

The key inputs are configured as pull-up inputs. Each switch is expected
to pull the pin low when pressed.

The linker script currently assumes:

- `FLASH`: `512K`
- `RAM`: `128K`

## Mode Selection

Hold all three keys long enough to enter selector mode. The LED is
driven low while the selector is active.

Choose a mode with the following sequence, then confirm with `K2`:

- `K1` -> Number mode
- `K3` -> Navigation mode
- `K1`, then `K3` -> Media mode
- Any other sequence -> Vim mode

Current LED feedback after confirmation:

- 1 blink: Vim
- 2 blinks: Number or Navigation
- 3 blinks: Media

## Key Map

### Vim Mode

| Gesture | Output |
| --- | --- |
| `K1` short | `i` |
| `K1` long | `I` |
| `K2` short | `Esc` |
| `K2` long | `.` |
| `K3` short | `a` |
| `K3` long | `A` |
| `K1 + K2` | `#` |
| `K1 + K3` | `%` |
| `K2 + K3` | `*` |

### Number Mode

| Gesture | Output |
| --- | --- |
| `K1` short | `1` |
| `K1` long | `4` |
| `K2` short | `2` |
| `K2` long | `5` |
| `K3` short | `3` |
| `K3` long | `6` |
| `K1 + K2` | `7` |
| `K1 + K3` | `8` |
| `K2 + K3` | `9` |
| `K1 + K2 + K3` | `0` |

### Navigation Mode

| Gesture | Output |
| --- | --- |
| `K1` short | Left Arrow |
| `K1` long | Home |
| `K2` short | Enter |
| `K2` long | Esc |
| `K3` short | Right Arrow |
| `K3` long | End |
| `K1 + K2` | Up Arrow |
| `K1 + K3` | Space |
| `K2 + K3` | Down Arrow |

### Media Mode

| Gesture | Output |
| --- | --- |
| `K1` short | Previous Track |
| `K2` short | Play/Pause |
| `K3` short | Next Track |
| `K1 + K2` | Volume Down |
| `K1 + K3` | Mute |
| `K2 + K3` | Volume Up |

## Build

The project is configured for the `thumbv7em-none-eabi` target.

Prerequisites:

- Rust toolchain with `thumbv7em-none-eabi`
- ARM GNU toolchain, especially `arm-none-eabi-objcopy`

Install the Rust target if needed:

```bash
rustup target add thumbv7em-none-eabi
```

Useful commands:

```bash
cargo check
cargo build
cargo build --release
make
```

Artifacts:

- ELF: `target/thumbv7em-none-eabi/release/cckeyboard`
- Binary: `target/firmware.bin`
- Intel HEX: `target/firmware.hex`

## Flashing

This repository does not include a board-specific flashing command.
Use the tool that matches your hardware setup, for example `probe-rs`,
`st-flash`, or an external bootloader workflow.

## Project Layout

- `src/main.rs`: application entry point and mode mapping
- `src/button.rs`: key scanning, debounce, long press, and chord logic
- `src/board.rs`: pin definitions and clock setup
- `src/usb_hid.rs`: USB HID keyboard and media transport
- `memory.x`: memory layout
- `.cargo/config.toml`: default build target and linker flags
- `Makefile`: release packaging helpers

## Validation

At minimum, validate:

- `cargo check`
- `cargo build --release`
- `make`

For input-related changes, verify on hardware:

- short press on `K1`, `K2`, `K3`
- long press on `K1`, `K2`, `K3`
- all two-key chords
- three-key selector entry
- mode selection and confirmation
- USB enumeration on the host
