#include "presage.h"

static void xfer_start_fnc(PurpleXfer *xfer) {
    PurpleAccount *account = purple_xfer_get_account(xfer);
    PurpleConnection *connection = purple_account_get_connection(account);
    Presage *presage = purple_connection_get_protocol_data(connection);
    presage_rust_send(account, rust_runtime, presage->tx_ptr, xfer->who, NULL, xfer);
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
        // this path is taken in drag-and-drop scenarios (filename already given)
        purple_xfer_request_accepted(xfer, filename);
    } else {
        // this path is taken in "send file…" scenarios (filename is unknown and we need to ask for it)
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
