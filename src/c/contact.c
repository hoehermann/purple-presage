#include "presage.h"

/*
 * This is called after a buddy has been added to the buddy list 
 * (i.e. by manual user interaction).
 */
void presage_add_buddy(PurpleConnection *connection, PurpleBuddy *buddy, PurpleGroup *group) {
    purple_prpl_got_user_status(purple_connection_get_account(connection), buddy->name, purple_primitive_get_id_from_type(PURPLE_STATUS_AVAILABLE) /* TODO: make user configurable */, NULL);
}
