#include "presage.h"

void presage_handle_text(PurpleConnection *connection, const char *who, const char *name, const char *group, const char *title, uint64_t sent, uint64_t timestamp, const char *body) {
    PurpleAccount *account = purple_connection_get_account(connection);

    // Signal is a plain-text protocol, but Pidgin expects HTML
    // NOTE: This turns newlines into br-tags which may mess up textual representation of QR-codes
    gchar *text = purple_markup_escape_text(body, -1);

    PurpleMessageFlags flags = 0;
    if (sent) {
        // special handling of messages sent by self incoming from remote for Spectrum
        // send-acknowledgements should be PURPLE_MESSAGE_SEND only (without PURPLE_MESSAGE_REMOTE_SEND)
        // for details, look into purple-whatsmeow
        flags |= PURPLE_MESSAGE_SEND | PURPLE_MESSAGE_REMOTE_SEND;
    } else {
        flags |= PURPLE_MESSAGE_RECV;
    }
    
    if (group == NULL) {
        // direct message
        presage_blist_update_buddy(account, who, name); // add to blist first for aliasing
        if (flags & PURPLE_MESSAGE_SEND) {
            // display message sent from own account (other device as well as local echo)
            // cannot use purple_serv_got_im since it sets the flag PURPLE_MESSAGE_RECV
            PurpleConversation *conv = purple_conversation_find_im_by_name(who, account);
            if (conv == NULL) {
                conv = purple_im_conversation_new(account, who); // MEMCHECK: caller takes ownership
            }
            purple_conv_im_write(purple_conversation_get_im_data(conv), who, text, flags, timestamp/1000);
        } else {
            purple_serv_got_im(connection, who, text, flags, timestamp/1000);
        }
    } else {
        // group message
        presage_blist_update_chat(account, group, title); // add to blist first for aliasing
        PurpleConversation *conv = purple_find_chat(connection, g_str_hash(group));
        if (conv == NULL) {
            conv = serv_got_joined_chat(connection, g_str_hash(group), title); // TODO: be really sure about setting the name to the topic here
            purple_conversation_set_data(conv, "name", g_strdup(group)); // MEMCHECK: this leaks, but there is no mechanism to stop it
            PurpleConvChat *conv_chat = purple_conversation_get_chat_data(conv);
            purple_conv_chat_set_nick(conv_chat, purple_account_get_username(account));
        }
        if (flags & PURPLE_MESSAGE_SEND) {
            // the backend does not include the username for sync messages
            who = purple_account_get_username(account);
        }
        purple_serv_got_chat_in(connection, g_str_hash(group), who, flags, text, timestamp);
    }

    g_free(text);
}
