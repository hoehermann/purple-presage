#pragma once

#include <purple.h>

#define PLUGIN_NAME "presage"

// these should be supplied by rust in some way
typedef struct _RustRuntime * RustRuntimePtr;
typedef struct _RustChannelTx * RustChannelPtr;
RustRuntimePtr presage_rust_init();
void presage_rust_destroy(RustRuntimePtr);
void presage_rust_link(RustRuntimePtr, RustChannelPtr, char *);

extern RustRuntimePtr rust_runtime;

typedef struct {
    PurpleAccount *account;
    RustChannelPtr tx_ptr;
    char *qrcode;
} Presage;

void presage_login(PurpleAccount *account);
void presage_close(PurpleConnection *pc);

