#include "presage.h"

// NOTE: These functions are the same for WhatsApp and Signal. Maybe move to a repository of common functions?

PurpleGroup * presage_blist_get_group() {
    PurpleGroup *group = purple_blist_find_group("Signal");
    if (!group) {
        group = purple_group_new("Signal"); // MEMCHECK: caller takes ownership
        purple_blist_add_group(group, NULL);
    }
    return group;
}

/*
 * Ensure buddy in the buddy list.
 * Updates alias non-destructively.
 */
PurpleBuddy * presage_blist_update_buddy(PurpleAccount *account, const char *who, const char *name) {
    PurpleBuddy *buddy = purple_blist_find_buddy(account, who);

    if (!buddy) {
        PurpleGroup *group = presage_blist_get_group();
        buddy = purple_buddy_new(account, who, name); // MEMCHECK: blist takes ownership
        purple_blist_add_buddy(buddy, NULL, group, NULL);
    }

    presage_blist_set_online(account, buddy);

    // update name after checking against local alias
    if (name != NULL && *name) {
        const char *local_alias = purple_buddy_get_local_buddy_alias(buddy);
        const char *server_alias = purple_blist_node_get_string(&buddy->node, "server_alias");
        if (local_alias == NULL) {
            // if no local alias exists, use the provided one
            purple_blist_alias_buddy(buddy, name);
        }
        if (!purple_strequal(local_alias, name) && !purple_strequal(server_alias, name)) {
            purple_serv_got_alias(purple_account_get_connection(account), who, name); // it seems buddy->server_alias is not persisted
            purple_blist_node_set_string(&buddy->node, "server_alias", name); // explicitly persist the new name so there is no name-change reported after a restart
        }
    }

    return buddy;
}

void presage_blist_set_online(PurpleAccount *account, PurpleBuddy *buddy) {
    purple_protocol_got_user_status(
        account, 
        purple_buddy_get_name(buddy), 
        purple_primitive_get_id_from_type(PURPLE_STATUS_AVAILABLE) /* TODO: make user configurable */, 
        NULL
    );
}

void presage_blist_buddies_all_set_online(PurpleAccount *account) {
    for (GSList * buddies = purple_blist_find_buddies(account, NULL); buddies != NULL; buddies = g_slist_delete_link(buddies, buddies)) {
        PurpleBuddy *buddy = buddies->data;
        presage_blist_set_online(account, buddy);
    }
}

/*
 * This is called after a buddy has been added to the buddy list 
 * (i.e. by manual user interaction).
 */
void presage_add_buddy(PurpleConnection *connection, PurpleBuddy *buddy, PurpleGroup *group) {
    presage_blist_set_online(purple_connection_get_account(connection), buddy);
}

// Group chat related functions

/*
 * Add group chat to blist. Updates existing group chat if found.
 */
void presage_blist_update_chat(PurpleAccount *account, const char *identifier, const char *topic) {
    g_return_if_fail(purple_account_is_connected(account));

    PurpleChat *chat = purple_blist_find_chat(account, identifier); // can only work if chat_info is defined

    if (chat == NULL) {
        GHashTable *comp = g_hash_table_new_full(g_str_hash, g_str_equal, NULL, g_free); // MEMCHECK: purple_chat_new takes ownership
        g_hash_table_insert(comp, "name", g_strdup(identifier)); // MEMCHECK: g_strdup'ed string released by GHashTable's value_destroy_func g_free (see above)
        chat = purple_blist_chat_new(account, topic, comp); // MEMCHECK: blist takes ownership
        PurpleGroup *group = presage_blist_get_group();
        purple_blist_add_chat(chat, group, NULL);
    }

    if (topic != NULL) {
        purple_blist_alias_chat(chat, topic);
    }
}

void presage_handle_contact(PurpleConnection *connection, const char *uuid, const char *name, const char *phone_number) {
    PurpleBuddy *buddy = presage_blist_update_buddy(purple_connection_get_account(connection), uuid, name);
    purple_blist_node_set_string(&buddy->node, "phone_number", phone_number);
}

void presage_tooltip_text(PurpleBuddy *buddy, PurpleNotifyUserInfo *info, gboolean full) {
    const char *server_alias = purple_blist_node_get_string(&buddy->node, "server_alias");
    if (server_alias != NULL) {
        purple_notify_user_info_add_pair(info, "Name", server_alias);
    }
    const char *phone_number = purple_blist_node_get_string(&buddy->node, "phone_number");
    if (phone_number != NULL) {
        purple_notify_user_info_add_pair(info, "Number", phone_number);
    }
}
