#include "presage.h"

/*
 * Copied from
 * https://github.com/hoehermann/libpurple-signald/blob/master/groups.c
 */
void presage_set_chat_topic(PurpleConnection *pc, int id, const char *topic) {
    /*
    Nothing to do here. For some reason, Pidgin only enables the "Alias..." 
    menu option in the conversation iff this callback is registered.
    */
}

/*
 * According to libpurple/prpl.h, this shall return a list of identifiers 
 * needed to join a group chat. By default, the first element of this list 
 * must be the identifying aspect, see purple_blist_find_chat in 
 * libpurple/blist.c.
 * 
 * bitlbee expects this function to be present.
 * 
 * Copied from
 * https://github.com/hoehermann/libpurple-signald/blob/master/groups.c
 */
GList * presage_chat_info(PurpleConnection *connection) {
    GList *infos = NULL;

    struct proto_chat_entry *pce;

    pce = g_new0(struct proto_chat_entry, 1); // MEMCHECK: infos takes ownership
    pce->label = "Identifier";
    pce->identifier = "name";
    pce->required = TRUE;
    infos = g_list_append(infos, pce);

    pce = g_new0(struct proto_chat_entry, 1); // MEMCHECK: infos takes ownership
    pce->label = "Group Name";
    pce->identifier = "topic";
    pce->required = TRUE;
    infos = g_list_append(infos, pce);

    return infos; // MEMCHECK: caller takes ownership
}

/*
 * The user wants to join a chat.
 * 
 * data is a table filled with the information needed to join the chat
 * as defined by chat_info_defaults. We only need the identifier.
 * 
 * Note: In purple, "name" is implicitly set to the roomlist room name in 
 * purple_roomlist_room_join, see libpurple/roomlist.c
 * 
 * Since group chat participation is handled by the main device, this function
 * does not actually send any requests to the server.
 */
void presage_join_chat(PurpleConnection *connection, GHashTable *data) {
    Presage *presage = purple_connection_get_protocol_data(connection);
    const char *identifier = g_hash_table_lookup(data, "name");
    const char *topic = g_hash_table_lookup(data, "topic");
    if (identifier != NULL) {
        PurpleAccount *account = purple_connection_get_account(connection);
        presage_blist_update_chat(account, identifier, topic); // add to blist first for aliasing
        PurpleConversation *conv = purple_find_chat(connection, g_str_hash(identifier));
        if (conv == NULL || (conv != NULL && purple_conversation_get_data(conv, "want-to-rejoin"))) {
            /*
            identifier is passed to purple_conversation_new as name which usually is the identifying aspect of a
            conversation (regardless if direct or chat). purple_conversation_autoset_title uses purple_chat_get_name 
            wich actually returns the alias for the title
            */
            conv = serv_got_joined_chat(connection, g_str_hash(identifier), identifier);
            if (purple_conversation_get_data(conv, "want-to-rejoin")) {
                // now that we did rejoin, remove the flag
                // directly accessing conv->data feels wrong, but there is no interface to do it another way
                g_hash_table_remove(conv->data, "want-to-rejoin");
            }
            if (conv != NULL) {
                // store the indentifer so it can be retrieved by get_chat_name
                purple_conversation_set_data(conv, "name", g_strdup(identifier)); // MEMCHECK: this leaks, but there is no mechanism to stop it
                // set our user's chat nick here as purple_conversation_new prefers the local alias over the username
                PurpleConvChat *conv_chat = purple_conversation_get_chat_data(conv);
                purple_conv_chat_set_nick(conv_chat, purple_account_get_username(account));
                // request list of participants
                presage_rust_get_group_members(rust_runtime, presage->tx_ptr, identifier);
            }
        }
    }
}

/*
 * Handle list of group members by adding participants to chat if it is currently active.
 * 
 * NOTE: We cannot selectively add missing users since it looks like on Spectrum 
 * only a remove-readd-cycle will trigger the display name resolution.
 */
void presage_handle_members(PurpleConnection *connection, const char *group, char **members, uint64_t length) {
    g_return_if_fail(members != NULL);

    PurpleConversation *conv = purple_find_chat(connection, g_str_hash(group));
    if (conv != NULL) {
        PurpleConvChat *conv_chat = purple_conversation_get_chat_data(conv);
        purple_conv_chat_clear_users(conv_chat);
        for (uint64_t i = 0; i < length; i++) {
            PurpleConvChatBuddyFlags flags = 0;
            purple_conv_chat_add_user(conv_chat, members[i], NULL, flags, FALSE);
        }
    } else {
        purple_debug_warning(PLUGIN_NAME, "got list of participants for a non-existent chat %s\n", group);
    }
}
