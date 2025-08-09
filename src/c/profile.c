#include "presage.h"

void presage_get_info(PurpleConnection *connection, const char *who) {
    PurpleAccount *account = purple_connection_get_account(connection);
    Presage *presage = purple_connection_get_protocol_data(connection);
    g_free(presage->profile);
    presage->profile = g_strdup(who);
    presage_rust_get_profile(account, rust_runtime, presage->tx_ptr, who);
}

void presage_show_info(PurpleConnection *connection, const char *uuid, const char *name, const char *phone_number) {
    Presage *presage = purple_connection_get_protocol_data(connection);
    if (purple_strequal(presage->profile, uuid)) {
        g_free(presage->profile);
        presage->profile = NULL;

        PurpleNotifyUserInfo *user_info = purple_notify_user_info_new();
        purple_notify_user_info_add_pair_plaintext(user_info, "UUID", uuid);
        purple_notify_user_info_add_pair_plaintext(user_info, "Name", name);
        purple_notify_user_info_add_pair_plaintext(user_info, "Phone Number", phone_number);
        purple_notify_userinfo(connection, uuid, user_info, NULL, NULL);
        purple_notify_user_info_destroy(user_info);
    }
}