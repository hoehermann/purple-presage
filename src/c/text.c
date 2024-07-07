// TODO: rename to receive so it is analogous to send.c

#include "presage.h"

void presage_handle_text(PurpleConnection *connection, const char *who, const char *name, const char *group, const char *title, PurpleMessageFlags flags, uint64_t timestamp, const char *body) {
    PurpleAccount *account = purple_connection_get_account(connection);

    // Signal is a plain-text protocol, but Pidgin expects HTML
    // NOTE: This turns newlines into br-tags which may mess up textual representation of QR-codes
    gchar *text = purple_markup_escape_text(body, -1);
    
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
        PurpleConversation *conv = purple_find_chat(connection, g_str_hash(group));
        if (conv == NULL) {
            // no conversation for this group chat
            // prepare a GHashTable with the group identifier because that is how join_chat is supposed to work in purple
            GHashTable * data = g_hash_table_new_full(g_str_hash, g_str_equal, NULL, NULL); // MEMCHECK: structure itself is released below
            g_hash_table_insert(data, "name", (void *)group); // MEMCHECK: key "name" is static, value group is released by caller
            g_hash_table_insert(data, "topic", (void *)title); // MEMCHECK: key "topic" is title, value title is released by caller
            presage_join_chat(connection, data);
            g_hash_table_destroy(data); // MEMCHECK: g_hash_table_insert above
        }
        if (flags & PURPLE_MESSAGE_SEND) {
            // the backend does not include the username for sync messages
            who = purple_account_get_username(account);
        }
        purple_serv_got_chat_in(connection, g_str_hash(group), who, flags, text, timestamp);
    }

    g_free(text);
}
