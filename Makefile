RUST_LIBS ?= $(shell cd src/rust/ && cargo rustc -- --print native-static-libs 2>&1 | grep -Po '(?<=native-static-libs:).+')
LDFLAGS ?= $(shell pkg-config --libs $(PKG_CONFIG_PURPLE_ARGS) purple) $(shell pkg-config --libs libqrencode) $(RUST_LIBS)

SUFFIX := so
ifeq ($(OS),Windows_NT)
    SUFFIX := dll
endif

libpresage.$(SUFFIX): Makefile
	$(MAKE) -C src/c
	$(MAKE) -C src/rust
	$(CC) -shared -o $@ -static-libgcc -Wl,--whole-archive src/c/purple-presage.a -Wl,--no-whole-archive src/rust/target/debug/libpurple_presage_backend.a $(LDFLAGS)

PLUGINDIR ?= $(shell pkg-config purple --variable=plugindir)

install: libpresage.$(SUFFIX)
	install -m 755 libpresage.$(SUFFIX) "$(PLUGINDIR)"

.PHONY: clean

clean:
	rm -f libpresage.$(SUFFIX)
	$(MAKE) -C src/c clean
	$(MAKE) -C src/rust clean
