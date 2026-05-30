TARGET = target/thumbv7em-none-eabi/release/cckeyboard
PREFIX = arm-none-eabi

all: bin hex

release:
	cargo build --release

debug:
	cargo build

bin: release
	$(PREFIX)-objcopy $(TARGET) -O binary target/firmware.bin
	@chmod a-x target/firmware.bin

hex: release
	$(PREFIX)-objcopy $(TARGET) -O ihex target/firmware.hex

clean:
	cargo clean

.PHONY: clean
