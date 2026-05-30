# Repository Guidelines

## Project Structure & Module Organization
This repository is a Rust firmware project for `stm32f411` that currently implements a three-key USB HID keyboard. The active binary entry point is `src/main.rs`, published as the `cckeyboard` binary in `Cargo.toml`. Input scanning and gesture detection live in `src/button.rs`, board-level GPIO and clock setup live in `src/board.rs`, and USB HID transport lives in `src/usb_hid.rs`. Linker and memory configuration are split between `memory.x`, `build.rs`, and `.cargo/config.toml`. Build artifacts are generated under `target/`. The `Makefile` wraps common firmware export steps for `.bin` and `.hex` outputs.

## Build, Test, and Development Commands
Use Cargo for normal development and `make` when you need deployable firmware images.

- `cargo check` validates the crate quickly for the default `thumbv7em-none-eabi` target.
- `cargo build` produces a debug firmware build.
- `cargo build --release` produces the optimized ELF at `target/thumbv7em-none-eabi/release/morse`.
- `make` runs the release build and exports both `target/firmware.bin` and `target/firmware.hex`.
- `make clean` removes Cargo build artifacts.

The `Makefile` expects the ARM GNU toolchain, especially `arm-none-eabi-objcopy`, to be installed.

## Coding Style & Naming Conventions
Follow Rust 2021 idioms and run `cargo fmt` before submitting changes. Formatting is constrained by `rustfmt.toml` to `max_width = 79`. Use 4-space indentation, `snake_case` for functions and local variables, and keep module/file names lowercase. Prefer explicit state machines for key scanning, mode selection, and HID dispatch rather than burying timing logic directly in `main`.

## Testing Guidelines
There is no test suite yet, and the binary target disables Rust unit tests. At minimum, contributors should run `cargo check` and `cargo build --release`. When touching startup, linker, USB, or release packaging code, also run `make` to verify binary/hex generation. For input changes, validate the three-key paths on hardware: short press, long press, two-key chords, three-key selector entry, and mode switching via `K1/K3` sequences confirmed by `K2`. Add host-side tests only for logic that can be isolated from MCU-specific code.

## Commit & Pull Request Guidelines
Current history uses short imperative subjects with a prefix, for example `init: template for stm32f411`. Keep that pattern: `<scope>: <change>`. Pull requests should include a brief summary, note any changes to GPIO assignments, USB HID behavior, target configuration, or memory layout, and list the commands used for validation. Include board-specific wiring notes when behavior depends on a particular STM32F411 board or switch layout.
