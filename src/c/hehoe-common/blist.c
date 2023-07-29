#include "blist.h"

void hehoe_blist_buddies_all_set_state(PurpleAccount *account, const gchar *status_str) {
    for (GSList * buddies = purple_find_buddies(account, NULL); buddies != NULL; buddies = g_slist_delete_link(buddies, buddies)) {
        PurpleBuddy *buddy = buddies->data;
        purple_prpl_got_user_status(account, buddy->name, status_str, NULL);
    }
}
