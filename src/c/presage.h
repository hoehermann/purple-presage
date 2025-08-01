#pragma once

#include <purple.h>
#include "purple-2.h"
#include <stdint.h>

#define PLUGIN_NAME "presage"

extern const char * PRESAGE_STARTUP_DELAY_SECONDS_OPTION;
extern const char * PRESAGE_ATTACHMENT_PATH_TEMPLATE_OPTION;

// https://github.com/LLNL/lbann/issues/117#issuecomment-334333286
#define MAKE_STR(x) _MAKE_STR(x)
#define _MAKE_STR(x) #x

// these should be supplied by rust in some way
// TODO: uint64_t should actually correspond to rust's usize
typedef struct _RustRuntime * RustRuntimePtr;
typedef struct _RustChannelTx * RustChannelPtr;
typedef struct _RustAttachment * RustAttachmentPtr;
RustRuntimePtr presage_rust_init();
void presage_rust_destroy(RustRuntimePtr);
void presage_rust_whoami(PurpleConnection *, RustRuntimePtr, RustChannelPtr);
void presage_rust_exit(PurpleConnection *, RustRuntimePtr, RustChannelPtr);
void presage_rust_send(PurpleConnection *, RustRuntimePtr, RustChannelPtr, const char *, const char *, PurpleXfer *);
void presage_rust_get_attachment(PurpleConnection *, RustRuntimePtr, RustChannelPtr, RustAttachmentPtr, PurpleXfer *);
void presage_rust_drop_attachment(RustAttachmentPtr);
void presage_rust_get_group_members(PurpleConnection *, RustRuntimePtr, RustChannelPtr, const char *);
void presage_rust_get_profile(PurpleConnection *, RustRuntimePtr, RustChannelPtr, const char *);
void presage_rust_list_groups(PurpleConnection *, RustRuntimePtr, RustChannelPtr);

extern RustRuntimePtr rust_runtime;

// structures for receiving messages from rust
typedef struct {
    char *key;
    char *title;
    char *description;
    uint32_t revision;
    char **members;
    size_t population;
} Group;
typedef struct {
    PurpleAccount *account;
    RustChannelPtr tx_ptr;
    char *qrcode;
    char *uuid;
    const PurpleDebugLevel debug;
    const PurpleConnectionError error;
    const int32_t connected;
    const uint32_t attachment_size;
    const uint64_t timestamp;
    const PurpleMessageFlags flags;
    char *who;
    char *name;
    char *phone_number;
    char *group;
    char *body;
    void *attachment_pointer_box;
    char *extension;
    char *filename;
    char *hash;
    Group *groups;
    size_t groups_length;
    PurpleXfer *xfer;
} Message;

// data regarding this connection
typedef struct {
    RustChannelPtr tx_ptr;
    PurpleRoomlist *roomlist;
    char *profile;
} Presage;

// procotol properties
GList * presage_status_types(PurpleAccount *account);
GList * presage_add_account_options(GList *account_options);

// connection
void presage_login(PurpleAccount *account);
void presage_close(PurpleConnection *pc);
void presage_account_error(PurpleAccount *account, PurpleConnectionError reason, const char *description);

// qrcode (linking and identification)
void presage_handle_qrcode(PurpleConnection * connection, const char *qrcode);
void presage_request_qrcode(PurpleConnection *connection);
void presage_handle_uuid(PurpleConnection *connection, const char *uuid);

// text messages
void presage_handle_text(PurpleConnection *connection, const char *who, const char *name, const char *group, PurpleMessageFlags sent, uint64_t timestamp_ms, const char *body);
int presage_send_im(PurpleConnection *connection, const char *who, const char *message, PurpleMessageFlags flags);
int presage_send_chat(PurpleConnection *connection, int id, const gchar *message, PurpleMessageFlags flags);

// contact management
void presage_add_buddy(PurpleConnection *connection, PurpleBuddy *buddy, PurpleGroup *group);
PurpleBuddy *presage_blist_update_buddy(PurpleAccount *account, const char *uuid, const char *name);
void presage_blist_set_online(PurpleAccount *account, PurpleBuddy *buddy);
void presage_blist_buddies_all_set_online(PurpleAccount *account);
void presage_blist_update_chat(PurpleAccount *account, const char *identifier, const char *topic);
void presage_handle_contact(PurpleConnection *connection, const char *uuid, const char *name, const char *phone_number);
void presage_tooltip_text(PurpleBuddy *buddy, PurpleNotifyUserInfo *info, gboolean full);
void presage_get_info(PurpleConnection *connection, const char *who);
void presage_show_info(PurpleConnection *connection, const char *uuid, const char *name, const char *phone_number);

// group management
void presage_set_chat_topic(PurpleConnection *connection, int id, const char *topic);
GList * presage_chat_info(PurpleConnection *connection);
void presage_join_chat(PurpleConnection *connection, GHashTable *data);
void presage_handle_groups(PurpleConnection *connection, const Group *groups, uint64_t length);
//void presage_handle_members(PurpleConnection *connection, const char *group, char **members, uint64_t length);
PurpleRoomlist * presage_roomlist_get_list(PurpleConnection *connection);

// attachments
void presage_handle_attachment(PurpleConnection *connection, const char *who, const char *chat, PurpleMessageFlags flags, uint64_t timestamp, RustAttachmentPtr attachment_pointer_box, uint64_t attachment_size, const char *hash, const char *filename, const char *extension);
void presage_send_file(PurpleConnection *connection, const gchar *who, const gchar *filename);
void presage_chat_send_file(PurpleConnection *connection, int id, const char *filename);
void presage_handle_xfer(PurpleXfer *xfer, PurpleMessageFlags flags, const char* error);
