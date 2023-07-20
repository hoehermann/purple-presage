#pragma once

#include <purple.h>
#include <stdint.h>

#define PLUGIN_NAME "presage"

// these should be supplied by rust in some way
typedef struct _RustRuntime * RustRuntimePtr;
typedef struct _RustChannelTx * RustChannelPtr;
RustRuntimePtr presage_rust_init();
void presage_rust_destroy(RustRuntimePtr);
void presage_rust_link(RustRuntimePtr, RustChannelPtr, const char *);
void presage_rust_whoami(RustRuntimePtr, RustChannelPtr);
void presage_rust_receive(RustRuntimePtr, RustChannelPtr);
void presage_rust_free(char *);

extern RustRuntimePtr rust_runtime;

typedef struct {
    PurpleAccount *account;
    RustChannelPtr tx_ptr;
    char *qrcode;
    char *uuid;
    const uint64_t timestamp;
    const uint64_t sent;
    char *who;
    char *group;
    char *body;
} Presage;

// connection
void presage_login(PurpleAccount *account);
void presage_close(PurpleConnection *pc);

// qrcode (linking and identification)
void presage_handle_qrcode(PurpleConnection * connection, const char *qrcode);
void presage_request_qrcode(PurpleConnection *connection);
void presage_handle_uuid(PurpleConnection *connection, const char *uuid);

// text messages
void presage_handle_text(PurpleConnection *connection, const char *who, const char *group, uint64_t sent, uint64_t timestamp, const char *text);
