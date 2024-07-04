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
    purple_notify_message(NULL, PURPLE_NOTIFY_MSG_INFO, "Not implemented", "Opening group chat not implemented.", NULL, NULL, NULL);
}
