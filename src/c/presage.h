#pragma once

#include <purple.h>

#define PLUGIN_NAME "presage"

// these should be supplied by rust in some way
typedef struct _RustRuntime * RustRuntimePtr;
typedef struct _RustChannelTx * RustChannelPtr;
RustRuntimePtr presage_rust_init();
void presage_rust_destroy(RustRuntimePtr);
void presage_rust_link(RustRuntimePtr, RustChannelPtr, const char *);

extern RustRuntimePtr rust_runtime;

typedef struct {
    PurpleAccount *account;
    RustChannelPtr tx_ptr;
    const char *qrcode;
    const char *uuid;
} Presage;

void presage_login(PurpleAccount *account);
void presage_close(PurpleConnection *pc);

void presage_handle_qrcode(PurpleConnection * connection, const char *qrcode);
void presage_request_qrcode(Presage *presage);
