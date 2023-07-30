#include "presage.h"

// TODO: COMMON function. Maybe move to submodule.
void presage_blist_buddies_all_set_state(PurpleAccount *account, const gchar *status_str) {
    for (GSList * buddies = purple_blist_find_buddies(account, NULL); buddies != NULL; buddies = g_slist_delete_link(buddies, buddies)) {
        PurpleBuddy *buddy = buddies->data;
        purple_protocol_got_user_status(account, purple_buddy_get_name(buddy), status_str, NULL);
    }
}

/*
 * This is called after a buddy has been added to the buddy list 
 * (i.e. by manual user interaction).
 */
void presage_add_buddy(PurpleConnection *connection, PurpleBuddy *buddy, PurpleGroup *group) {
    purple_protocol_got_user_status(purple_connection_get_account(connection), purple_buddy_get_name(buddy), purple_primitive_get_id_from_type(PURPLE_STATUS_AVAILABLE) /* TODO: make user configurable */, NULL);
}
