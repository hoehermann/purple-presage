BACKEND_VERSION ?= $(shell cargo metadata --format-version 1 --no-deps --manifest-path ../rust/Cargo.toml | jq --join-output '.packages[0].version')
COMMIT_DATE ?= $(shell git log -1 --date=format:'%Y%m%d' --format='%ad')
PRESAGE_COMMIT ?= $(shell cargo metadata --format-version 1 --no-deps --manifest-path ../rust/Cargo.toml | jq --join-output '.packages[0].dependencies[]|select(.name=="presage").source|split("=")[1]')
REVISION_COUNT ?= $(shell git rev-list --count HEAD)
PLUGIN_VERSION ?= $(BACKEND_VERSION)_$(COMMIT_DATE)_$(REVISION_COUNT)_$(PRESAGE_COMMIT)

CFLAGS ?= -fPIC -DPLUGIN_VERSION=$(PLUGIN_VERSION) $(shell pkg-config $(PKG_CONFIG_PURPLE_ARGS) --cflags purple) $(shell pkg-config --cflags libqrencode)

SRCS := $(wildcard *.c)
OBJS := $(SRCS:.c=.o)

purple-presage.a: $(OBJS)
	ar rcs $@ $(OBJS)

%.o: %.c Makefile
	$(CC) $(CFLAGS) -c $< -o $@

.PHONY: clean

clean:
	rm -f $(OBJS)
