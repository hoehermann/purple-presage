LDFLAGS := $(shell pkg-config --libs purple glib-2.0 libqrencode) $(shell cd src/rust/ && cargo rustc -- --print native-static-libs 2>&1 | grep -Po '(?<=native-static-libs:).+')

libpresage.so: src/c/purple-presage.a src/rust/target/debug/libpurple_presage_backend.a Makefile
	$(CC) -shared -o $@ -Wl,--whole-archive src/c/purple-presage.a -Wl,--no-whole-archive src/rust/target/debug/libpurple_presage_backend.a $(LDFLAGS)

src/c/purple-presage.a:
	$(MAKE) -C src/c

src/rust/target/debug/libpurple_presage_backend.a:
	$(MAKE) -C src/rust

PLUGINDIR := $(shell pkg-config purple --variable=plugindir)

install: libpresage.so
	install -m 755 libpresage.so "$(PLUGINDIR)"
	