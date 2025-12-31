RUST_LIBS ?= $(shell cd src/rust/ && cargo rustc -- --print native-static-libs 2>&1 | grep -Po '(?<=native-static-libs:).+')
LDFLAGS ?= $(shell pkg-config --libs $(PKG_CONFIG_PURPLE_ARGS) purple) $(shell pkg-config --libs libqrencode) $(RUST_LIBS)

SUFFIX := so
ifeq ($(OS),Windows_NT)
    SUFFIX := dll
endif

libpresage.$(SUFFIX): src/c/purple-presage.a src/rust/target/i686-pc-windows-gnu/debug/libpurple_presage_backend.a Makefile
	$(CC) -shared -o $@ -Wl,--whole-archive src/c/purple-presage.a -Wl,--no-whole-archive src/rust/target/i686-pc-windows-gnu/debug/libpurple_presage_backend.a $(LDFLAGS)

.PHONY: clean src/c/purple-presage.a src/rust/target/i686-pc-windows-gnu/debug/libpurple_presage_backend.a

src/c/purple-presage.a:
	$(MAKE) -C src/c

src/rust/target/i686-pc-windows-gnu/debug/libpurple_presage_backend.a:
	$(MAKE) -C src/rust

PLUGINDIR ?= $(shell pkg-config purple --variable=plugindir)

install: libpresage.$(SUFFIX)
	install -m 755 libpresage.so "$(PLUGINDIR)"

clean:
	rm -f libpresage.$(SUFFIX)
	$(MAKE) -C src/c clean
	$(MAKE) -C src/rust clean
