#include <inttypes.h>
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
    }
}

void presage_roomlist_populate(PurpleConnection *connection, const Group *groups, uint64_t length) {
    g_return_if_fail(groups != NULL || length == 0);

    Presage *presage = purple_connection_get_protocol_data(connection);
    PurpleRoomlist *roomlist = presage->roomlist;
    if (roomlist != NULL) {
        for (uint64_t i = 0; i < length; i++) {
            PurpleRoomlistRoom *room = purple_roomlist_room_new(PURPLE_ROOMLIST_ROOMTYPE_ROOM, groups[i].key, NULL); // MEMCHECK: roomlist will take ownership 
            // purple_roomlist_room_new sets the room's identifier
            purple_roomlist_room_add_field(roomlist, room, groups[i].title); // MEMCHECK: value is strdup'ed in callee
            purple_roomlist_room_add_field(roomlist, room, groups[i].description); // MEMCHECK: value is strdup'ed in callee
            purple_roomlist_room_add(roomlist, room);
        }
        purple_roomlist_set_in_progress(roomlist, FALSE);
        purple_roomlist_unref(roomlist); // unref here, roomlist may remain in ui
        presage->roomlist = NULL;
    }
}

/*
 * This callback handles information about groups:
 * * The information might be a list of all known groups (without information about members). Needed for populating the room-list.
 * * The information might be a list of exactly one group (with information about members). Needed for populating the chat participant list.
 */
void presage_handle_groups(PurpleConnection *connection, const Group *groups, uint64_t length) {
    g_return_if_fail(groups != NULL);

    presage_roomlist_populate(connection, groups, length);

    for (uint64_t i = 0; i < length; i++) {
        purple_debug_info(PLUGIN_NAME, "got group %s „%s“ with %" PRIu64 " members at %p\n", groups[i].key, groups[i].title, groups[i].population, groups[i].members);
        presage_blist_update_chat(purple_connection_get_account(connection), groups[i].key, groups[i].title);
        if (groups[i].members != NULL) {
            presage_handle_members(connection, groups[i].key, groups[i].members, groups[i].population);
        }
    }
}

/*
 * This requests a list of rooms representing the Signal group chats.
 * The request is asynchronous. Response is handled by presage_handle_groups.
 * 
 * A purple room has an identifying name – for Singal that is the Group Master Key.
 * A purple room has a list of fields – in our case the Signal group title and description 
 * (could also be revision, number of participants).
 * 
 * Some services like spectrum expect the human readable group name field key to be "topic", 
 * see RoomlistProgress in https://github.com/SpectrumIM/spectrum2/blob/518ba5a/backends/libpurple/main.cpp#L1997
 * In purple, the roomlist field "name" gets overwritten in purple_roomlist_room_join, see libpurple/roomlist.c.
 */
PurpleRoomlist * presage_roomlist_get_list(PurpleConnection *connection) {
    PurpleAccount *account = purple_connection_get_account(connection);
    PurpleRoomlist *roomlist = purple_roomlist_new(account); // MEMCHECK: caller takes ownership
    purple_roomlist_set_in_progress(roomlist, TRUE);
    GList *fields = NULL;
    fields = g_list_append(fields, purple_roomlist_field_new( // MEMCHECK: fields takes ownership
        PURPLE_ROOMLIST_FIELD_STRING, "Name", "topic", FALSE
    ));
    fields = g_list_append(fields, purple_roomlist_field_new( // MEMCHECK: fields takes ownership
        PURPLE_ROOMLIST_FIELD_STRING, "Description", "description", FALSE
    ));
    purple_roomlist_set_fields(roomlist, fields);
    Presage *presage = purple_connection_get_protocol_data(connection);
    presage->roomlist = roomlist; // store the pointer to the roomlist so presage_handle_groups can write to it
    presage_rust_list_groups(rust_runtime, presage->tx_ptr);
    return roomlist;
}
