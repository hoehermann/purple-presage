RUST_LIBS ?= $(shell cd src/rust/ && cargo rustc -- --print native-static-libs 2>&1 | grep -Po '(?<=native-static-libs:).+')
LDFLAGS ?= $(shell pkg-config --libs $(PKG_CONFIG_PURPLE_ARGS) purple) $(shell pkg-config --libs libqrencode gdk-pixbuf-2.0) $(RUST_LIBS)

export LIBRARY_PREFIX ?= lib
export LIBRARY_SUFFIX ?= a
SHARED_SUFFIX := so
ifeq ($(OS),Windows_NT)
    SHARED_SUFFIX := dll
    #export RUST_TARGET := i686-win7-windows-gnu
    #export CARGO_BUILD_FLAGS := --target $(RUST_TARGET) -Zbuild-std
    # thanks to https://blog.bemyak.net/dev/building-rust-tier-3-on-stable/ for the hint
endif

presage: libpresage.$(SHARED_SUFFIX)

libpresage.dll: src/c/$(LIBRARY_PREFIX)purple-presage.$(LIBRARY_SUFFIX) src/rust/target/$(RUST_TARGET)/debug/$(LIBRARY_PREFIX)purple_presage_backend.$(LIBRARY_SUFFIX) Makefile
	$(LINK_PROGRAM) /DLL /MACHINE:X86 /OUT:$@ src/c/*.obj src/rust/target/$(RUST_TARGET)/debug/$(LIBRARY_PREFIX)purple_presage_backend.$(LIBRARY_SUFFIX)

libpresage.so: src/c/$(LIBRARY_PREFIX)purple-presage.$(LIBRARY_SUFFIX) src/rust/target/$(RUST_TARGET)/debug/$(LIBRARY_PREFIX)purple_presage_backend.$(LIBRARY_SUFFIX) Makefile
	$(CC) -shared -o $@ -static-libgcc -Wl,--whole-archive src/c/purple-presage.$(LIBRARY_SUFFIX) -Wl,--no-whole-archive src/rust/target/$(RUST_TARGET)/debug/libpurple_presage_backend.$(LIBRARY_SUFFIX) $(LDFLAGS)

.PHONY: clean src/c/$(LIBRARY_PREFIX)purple-presage.$(LIBRARY_SUFFIX) src/rust/target/$(RUST_TARGET)/debug/$(LIBRARY_PREFIX)purple_presage_backend.$(LIBRARY_SUFFIX)

src/c/$(LIBRARY_PREFIX)purple-presage.$(LIBRARY_SUFFIX):
	$(MAKE) -C src/c

src/rust/target/$(RUST_TARGET)/debug/$(LIBRARY_PREFIX)purple_presage_backend.$(LIBRARY_SUFFIX):
	echo src/rust/target/$(RUST_TARGET)/debug/$(LIBRARY_PREFIX)purple_presage_backend.$(LIBRARY_SUFFIX) fehlt angeblich
#$(MAKE) -C src/rust

PLUGIN_DIR ?= $(shell pkg-config purple --variable=plugindir)
DATA_ROOT_DIR ?= $(shell pkg-config purple --variable=datarootdir)
DIR_PERM = 0755
FILE_PERM = 0644
PIXMAP_SIZES = 16 22 48 64 512

install:
	mkdir -m $(DIR_PERM) -p "$(DESTDIR)$(PLUGIN_DIR)"
	install -m $(FILE_PERM) libpresage.$(SHARED_SUFFIX) "$(DESTDIR)$(PLUGIN_DIR)/"
	$(foreach size,$(PIXMAP_SIZES),mkdir -m $(DIR_PERM) -p "$(DESTDIR)$(DATA_ROOT_DIR)/pixmaps/pidgin/protocols/$(size)" ;)
	$(foreach size,$(PIXMAP_SIZES),install -m $(FILE_PERM) assets/pixmaps/pidgin/protocols/$(size)/signal.png "$(DESTDIR)$(DATA_ROOT_DIR)/pixmaps/pidgin/protocols/$(size)/" ;)
clean:
	rm -f libpresage.$(SHARED_SUFFIX)
	$(MAKE) -C src/c clean
	$(MAKE) -C src/rust clean
