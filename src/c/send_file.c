#include "presage.h"

static void xfer_start_fnc(PurpleXfer *xfer) {
    PurpleAccount *account = purple_xfer_get_account(xfer);
    PurpleConnection *connection = purple_account_get_connection(account);
    Presage *presage = purple_connection_get_protocol_data(connection);
    const char *who = xfer->who;
    if (strlen(who) == 36 && who[8] == '-' && who[13] == '-' && who[18] == '-' && who[23] == '-') {
        // destination looks like a UUID, send to a contact
        presage_rust_send_contact(rust_runtime, presage->tx_ptr, xfer->who, NULL, xfer);
    } else {
        presage_rust_send_group(rust_runtime, presage->tx_ptr, xfer->who, NULL, xfer);
    }
}

static void presage_xfer_send_init(PurpleXfer *xfer) {
    purple_xfer_set_start_fnc(xfer, xfer_start_fnc);
    purple_xfer_start(xfer, -1, NULL, 0);
}

void xfer_new(PurpleConnection *connection, const char *destination, intptr_t destination_type, const char *filename) {
    PurpleAccount *account = purple_connection_get_account(connection);
    PurpleXfer *xfer = purple_xfer_new(account, PURPLE_XFER_TYPE_SEND, destination);
    purple_xfer_set_init_fnc(xfer, presage_xfer_send_init);
    if (filename && *filename) {
        purple_xfer_request_accepted(xfer, filename);
    } else {
        purple_xfer_request(xfer);
    }
}

void presage_send_file(PurpleConnection *connection, const gchar *who, const gchar *filename) {
    xfer_new(connection, who, PURPLE_CONV_TYPE_IM, filename);
}

void presage_chat_send_file(PurpleConnection *connection, int id, const char *filename) {
    PurpleConversation *conv = purple_find_chat(connection, id);
    g_return_if_fail(conv != NULL);
    const gchar *group = purple_conversation_get_data(conv, "name");
    g_return_if_fail(group != NULL);
    xfer_new(connection, group, PURPLE_CONV_TYPE_CHAT, filename);
}

void presage_handle_xfer(PurpleXfer *xfer, PurpleMessageFlags flags, const char* error) {
    if (flags & PURPLE_MESSAGE_ERROR) {
        PurpleAccount *account = purple_xfer_get_account(xfer);
        const char *destination = purple_xfer_get_remote_user(xfer);
        purple_xfer_error(purple_xfer_get_type(xfer), account, destination, error); 
        purple_xfer_cancel_local(xfer);
    } else {
        purple_xfer_set_bytes_sent(xfer, purple_xfer_get_size(xfer));
        purple_xfer_set_completed(xfer, TRUE);
    }
}