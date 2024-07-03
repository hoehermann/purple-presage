#include "presage.h"

int presage_send_im(PurpleConnection *connection, const char *who, const char *message, PurpleMessageFlags flags) {
    // Strip HTML similar to these reasons: https://github.com/majn/telegram-purple/issues/12 and https://github.com/majn/telegram-purple/commit/fffe751
    char *msg = purple_markup_strip_html(message); // NOTE: This turns newlines into spaces and <br> tags into newlines
    Presage *presage = purple_connection_get_protocol_data(connection);
    presage_rust_send_contact(rust_runtime, presage->tx_ptr, who, msg);
    g_free(msg);
    return 1; // boldly assumes message has been sent successfully
    // TODO: have various user-configurable ways of displaying success
}

int presage_send_chat(PurpleConnection *connection, int id, const gchar *message, PurpleMessageFlags flags) {
    Presage *presage = purple_connection_get_protocol_data(connection);
    PurpleConversation *conv = purple_find_chat(connection, id);
    if (conv != NULL) {
        gchar *group = (gchar *)purple_conversation_get_data(conv, "name");
        if (group != NULL) {
            // Strip HTML similar to these reasons: https://github.com/majn/telegram-purple/issues/12 and https://github.com/majn/telegram-purple/commit/fffe751
            char *msg = purple_markup_strip_html(message); // NOTE: This turns newlines into spaces and <br> tags into newlines
            presage_rust_send_group(rust_runtime, presage->tx_ptr, group, message);
            g_free(msg);
            // Group chats need an explicit local echo since the implicit echo is implemented for direct messages only.
            // See https://keep.imfreedom.org/pidgin/pidgin/file/v2.14.12/libpurple/conversation.c#l191.
            // TODO: only echo locally if the message has actually been sent
            PurpleConvChat *conv_chat = purple_conversation_get_chat_data(conv);
            PurpleAccount *account = purple_conversation_get_account(conv);
            purple_conv_chat_write(conv_chat, purple_account_get_username(account), message, flags, time(NULL));
        }
    }
    return 1; // boldly assumes message has been sent successfully
}