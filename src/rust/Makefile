ifeq ($(OS),Windows_NT)
    RUST_TARGET ?= --target i686-pc-windows-gnu
    #export RUSTFLAGS = -C target-feature=+crt-static
endif

target/debug/libpurple_presage_backend.a: Makefile Cargo.lock Cargo.toml src/attachment.rs src/bridge.rs src/bridge_commands.rs src/bridge_structs.rs src/contacts.rs src/core.rs src/lib.rs src/receive.rs src/send.rs src/structs.rs
	cargo build --profile dev $(RUST_TARGET)

.PHONY: clean

clean:
	cargo clean --profile dev
