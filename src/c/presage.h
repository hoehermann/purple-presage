#pragma once

#include "hehoe-purple2and3/purple.h"
#include <stdint.h>

#define PLUGIN_NAME "presage"

// https://github.com/LLNL/lbann/issues/117#issuecomment-334333286
#define MAKE_STR(x) _MAKE_STR(x)
#define _MAKE_STR(x) #x

// these should be supplied by rust in some way
typedef struct _RustRuntime * RustRuntimePtr;
typedef struct _RustChannelTx * RustChannelPtr;
RustRuntimePtr presage_rust_init();
void presage_rust_destroy(RustRuntimePtr);
void presage_rust_link(RustRuntimePtr, RustChannelPtr, const char *);
void presage_rust_whoami(RustRuntimePtr, RustChannelPtr);
void presage_rust_initial_sync(RustRuntimePtr, RustChannelPtr);
void presage_rust_receive(RustRuntimePtr, RustChannelPtr);
void presage_rust_exit(RustRuntimePtr, RustChannelPtr);
void presage_rust_send_contact(RustRuntimePtr, RustChannelPtr, const char *, const char *);
void presage_rust_send_group(RustRuntimePtr, RustChannelPtr, const char *, const char *);
void presage_rust_free(char *);

extern RustRuntimePtr rust_runtime;

// TODO: generate this struct declaration automatically from rust declaration
typedef struct {
    PurpleAccount *account;
    RustChannelPtr tx_ptr;
    char *qrcode;
    char *uuid;
    const int32_t debug;
    const int32_t error;
    const int32_t connected;
    const int32_t padding;
    const uint64_t timestamp;
    const uint64_t sent;
    char *who;
    char *name;
    char *group;
    char *title;
    char *body;
} Presage;

// procotol properties
GList * presage_status_types(PurpleAccount *account);

// connection
void presage_login(PurpleAccount *account);
void presage_close(PurpleConnection *pc);

// qrcode (linking and identification)
void presage_handle_qrcode(PurpleConnection * connection, const char *qrcode);
void presage_request_qrcode(PurpleConnection *connection);
void presage_handle_uuid(PurpleConnection *connection, const char *uuid);

// text messages
void presage_handle_text(PurpleConnection *connection, const char *who, const char *name, const char *group, const char *title, uint64_t sent, uint64_t timestamp, const char *body);
int presage_send_im(PurpleConnection *connection, const char *who, const char *message, PurpleMessageFlags flags);
int presage_send_chat(PurpleConnection *connection, int id, const gchar *message, PurpleMessageFlags flags);

// contact management
void presage_add_buddy(PurpleConnection *connection, PurpleBuddy *buddy, PurpleGroup *group);
void presage_blist_buddies_all_set_online(PurpleAccount *account);
void presage_blist_update_buddy(PurpleAccount *account, const char *uuid, const char *name);
void presage_blist_set_online(PurpleAccount *account, PurpleBuddy *buddy);
void presage_blist_buddies_all_set_online(PurpleAccount *account);
void presage_blist_update_chat(PurpleAccount *account, const char *identifier, const char *topic);

// group management
void presage_set_chat_topic(PurpleConnection *pc, int id, const char *topic);
GList * presage_chat_info(PurpleConnection *connection);
void presage_join_chat(PurpleConnection *connection, GHashTable *data);
