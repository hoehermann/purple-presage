target/$(RUST_TARGET)/debug/$(LIBRARY_PREFIX)purple_presage_backend.$(LIBRARY_SUFFIX): Makefile Cargo.lock Cargo.toml src/attachment.rs src/bridge.rs src/bridge_commands.rs src/bridge_structs.rs src/contacts.rs src/core.rs src/lib.rs src/receive.rs src/send.rs src/structs.rs src/debug.rs
	cargo build --profile dev $(CARGO_BUILD_FLAGS)

.PHONY: clean

clean:
	cargo clean --profile dev --target $(RUST_TARGET)
	$(REMOVE_PROGRAM) target/$(RUST_TARGET)/debug/$(LIBRARY_PREFIX)purple_presage_backend.$(LIBRARY_SUFFIX)
