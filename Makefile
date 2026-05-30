TARGET = target/thumbv7em-none-eabi/release/morse
PREFIX = arm-none-eabi

all: release bin hex

release:
	cargo build --release

debug: bin hex
	cargo build

bin: release
	$(PREFIX)-objcopy $(TARGET) -O binary target/firmware.bin

hex: release
	$(PREFIX)-objcopy $(TARGET) -O ihex target/firmware.hex

clean:
	cargo clean

.PHONY: clean
