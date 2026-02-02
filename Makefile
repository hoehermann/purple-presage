RUST_LIBS ?= $(shell cd src/rust/ && cargo rustc -- --print native-static-libs 2>&1 | grep -Po '(?<=native-static-libs:).+')
LDFLAGS ?= $(shell pkg-config --libs $(PKG_CONFIG_PURPLE_ARGS) purple) $(shell pkg-config --libs libqrencode gdk-pixbuf-2.0) $(RUST_LIBS)

SUFFIX := so
ifeq ($(OS),Windows_NT)
    SUFFIX := dll
endif

libpresage.$(SUFFIX): src/c/purple-presage.a src/rust/target/debug/libpurple_presage_backend.a Makefile
	$(CC) -shared -o $@ -static-libgcc -Wl,--whole-archive src/c/purple-presage.a -Wl,--no-whole-archive src/rust/target/debug/libpurple_presage_backend.a $(LDFLAGS)

.PHONY: clean src/c/purple-presage.a src/rust/target/debug/libpurple_presage_backend.a

src/c/purple-presage.a:
	$(MAKE) -C src/c

src/rust/target/debug/libpurple_presage_backend.a:
	$(MAKE) -C src/rust

PLUGIN_DIR ?= $(shell pkg-config purple --variable=plugindir)
DATA_ROOT_DIR ?= $(shell pkg-config purple --variable=datarootdir)
DIR_PERM = 0755
FILE_PERM = 0644
PIXMAP_SIZES = 16 22 48 64 512

install:
	mkdir -m $(DIR_PERM) -p "$(DESTDIR)$(PLUGIN_DIR)"
	install -m $(FILE_PERM) libpresage.so "$(DESTDIR)$(PLUGIN_DIR)/"
	$(foreach size,$(PIXMAP_SIZES),mkdir -m $(DIR_PERM) -p "$(DESTDIR)$(DATA_ROOT_DIR)/pixmaps/pidgin/protocols/$(size)" ;)
	$(foreach size,$(PIXMAP_SIZES),install -m $(FILE_PERM) assets/pixmaps/pidgin/protocols/$(size)/signal.png "$(DESTDIR)$(DATA_ROOT_DIR)/pixmaps/pidgin/protocols/$(size)/" ;)
clean:
	rm -f libpresage.$(SUFFIX)
	$(MAKE) -C src/c clean
	$(MAKE) -C src/rust clean
