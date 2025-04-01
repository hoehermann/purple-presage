#include "presage.h"

int presage_send_im(PurpleConnection *connection, const char *who, const char *message, PurpleMessageFlags flags) {
    // strip HTML similar to these reasons: https://github.com/majn/telegram-purple/issues/12 and https://github.com/majn/telegram-purple/commit/fffe751
    char *msg = purple_markup_strip_html(message); // NOTE: This turns newlines into spaces and <br> tags into newlines
    Presage *presage = purple_connection_get_protocol_data(connection);
    presage_rust_send(rust_runtime, presage->tx_ptr, who, msg, NULL);
    g_free(msg);
    return 0; // do not report an error here; also no local echo since the rust part is expected inject the message
}

int presage_send_chat(PurpleConnection *connection, int id, const gchar *message, PurpleMessageFlags flags) {
    Presage *presage = purple_connection_get_protocol_data(connection);
    PurpleConversation *conv = purple_find_chat(connection, id);
    if (conv != NULL) {
        gchar *group = (gchar *)purple_conversation_get_data(conv, "name");
        if (group != NULL) {
            // strip HTML similar to these reasons: https://github.com/majn/telegram-purple/issues/12 and https://github.com/majn/telegram-purple/commit/fffe751
            char *msg = purple_markup_strip_html(message); // NOTE: This turns newlines into spaces and <br> tags into newlines
            presage_rust_send(rust_runtime, presage->tx_ptr, group, msg, NULL);
            g_free(msg);
        }
    }
    return 0; // do not report an error here; also no local echo since the rust part is expected inject the message
}