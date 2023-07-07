#pragma once

#include <purple.h>

#define PLUGIN_NAME "presage"

typedef struct {} RustRuntime;
typedef struct {} RustChannelTx;

typedef struct {
    PurpleAccount *account;
    RustChannelTx *tx_ptr;
    char *qrcode;
} Presage;

extern RustRuntime * rust_runtime;

void presage_login(PurpleAccount *account);
void presage_close(PurpleConnection *pc);

RustRuntime * presage_rust_init();
void presage_rust_destroy(RustRuntime *);
void presage_rust_link(RustRuntime *, RustChannelTx *, char *);
