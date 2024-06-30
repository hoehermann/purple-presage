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