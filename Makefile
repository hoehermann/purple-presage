RUST_LIBS ?= $(shell cd src/rust/ && cargo rustc -- --print native-static-libs 2>&1 | grep -Po '(?<=native-static-libs:).+')
LDFLAGS ?= $(shell pkg-config --libs $(PKG_CONFIG_PURPLE_ARGS) purple) $(shell pkg-config --libs libqrencode gdk-pixbuf-2.0) $(RUST_LIBS)

SHARED_SUFFIX := so
export REMOVE_PROGRAM := rm -f
ifeq ($(OS),Windows_NT)
    SHARED_SUFFIX := dll
    export LIBRARY_SUFFIX ?= lib
    export REMOVE_PROGRAM := del
    LINK_PROGRAM ?= link
else
    export LIBRARY_SUFFIX ?= a
    LIBRARY_PREFIX ?= lib
endif

presage: $(LIBRARY_PREFIX)presage.$(SHARED_SUFFIX)

presage.dll: src/c/$(LIBRARY_PREFIX)purple-presage.$(LIBRARY_SUFFIX) src/rust/target/$(RUST_TARGET)/debug/$(LIBRARY_PREFIX)purple_presage_backend.$(LIBRARY_SUFFIX) Makefile
	$(LINK_PROGRAM) /DLL /MACHINE:X86 /OUT:$@ src/c/*.obj src/rust/target/$(RUST_TARGET)/debug/$(LIBRARY_PREFIX)purple_presage_backend.$(LIBRARY_SUFFIX) libpurple.lib glib-2.0.lib libcrypto.lib advapi32.lib qrencode.lib ntdll.lib Userenv.lib Crypt32.lib Bcrypt.lib

libpresage.$(SHARED_SUFFIX): src/c/$(LIBRARY_PREFIX)purple-presage.$(LIBRARY_SUFFIX) src/rust/target/$(RUST_TARGET)/debug/$(LIBRARY_PREFIX)purple_presage_backend.$(LIBRARY_SUFFIX) Makefile
	$(CC) -shared -o $@ -static-libgcc -Wl,--whole-archive src/c/purple-presage.$(LIBRARY_SUFFIX) -Wl,--no-whole-archive src/rust/target/$(RUST_TARGET)/debug/libpurple_presage_backend.$(LIBRARY_SUFFIX) $(LDFLAGS)

.PHONY: clean src/c/$(LIBRARY_PREFIX)purple-presage.$(LIBRARY_SUFFIX) src/rust/target/$(RUST_TARGET)/debug/$(LIBRARY_PREFIX)purple_presage_backend.$(LIBRARY_SUFFIX)

src/c/$(LIBRARY_PREFIX)purple-presage.$(LIBRARY_SUFFIX):
	$(MAKE) -C src/c

src/rust/target/$(RUST_TARGET)/debug/$(LIBRARY_PREFIX)purple_presage_backend.$(LIBRARY_SUFFIX):
	$(MAKE) -C src/rust

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
	$(REMOVE_PROGRAM) $(LIBRARY_PREFIX)presage.$(SHARED_SUFFIX)
	$(MAKE) -C src/c clean
	$(MAKE) -C src/rust clean
