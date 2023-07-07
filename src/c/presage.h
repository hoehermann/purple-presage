#pragma once

#include <purple.h>

#define PLUGIN_NAME "presage"

// these should be supplied by rust in some way
typedef struct {} RustRuntime;
typedef struct {} RustChannelTx;
RustRuntime * presage_rust_init();
void presage_rust_destroy(RustRuntime *);
void presage_rust_link(RustRuntime *, RustChannelTx *, char *);

extern RustRuntime * rust_runtime;

typedef struct {
    PurpleAccount *account;
    RustChannelTx *tx_ptr;
    char *qrcode;
} Presage;

void presage_login(PurpleAccount *account);
void presage_close(PurpleConnection *pc);

