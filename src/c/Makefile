PLUGIN_VERSION :=$(shell cargo metadata --format-version 1 --no-deps --manifest-path ../../src/rust/Cargo.toml | jq -j .packages[0].version)
CFLAGS := -fPIC -DPLUGIN_VERSION=$(PLUGIN_VERSION) $(shell pkg-config --cflags purple glib-2.0)

SRCS := $(wildcard *.c)
OBJS := $(SRCS:.c=.o)

purple-presage.a: $(OBJS)
	ar rcs $@ $(OBJS)

%.o: %.c Makefile
	$(CC) $(CFLAGS) -c $< -o $@
