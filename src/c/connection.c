#include "presage.h"

/*
 * The rust part announced that its main function has terminated (via a message with connected < 0).
 * This runs on the main thread. Since the rust → C bridge queue preserves order,
 * all messages the rust part sent before terminating have already been processed.
 */
void presage_handle_rust_main_finished(PurpleAccount *account) {
    PurpleConnection *connection = purple_account_get_connection(account);
    if (connection == NULL) {
        purple_debug_info(PLUGIN_NAME, "rust runtime has finished after connection ceased to exist.\n");
    } else if (PURPLE_CONNECTION_STATE_DISCONNECTED == purple_connection_get_state(connection)) {
        purple_debug_info(PLUGIN_NAME, "rust runtime has finished as expected.\n");
    } else {
        purple_connection_error(connection, PURPLE_CONNECTION_ERROR_OTHER_ERROR, "rust runtime has finished unexpectedly.");
    }
}

void presage_login(PurpleAccount *account) {
    purple_debug_info(PLUGIN_NAME, "login for account: %p\n", account);
    g_return_if_fail(rust_runtime != NULL);
    purple_debug_info(PLUGIN_NAME, "rust_runtime is at %p\n", rust_runtime);
    PurpleConnection *connection = purple_account_get_connection(account);
    // this protocol does not support anything special right now
    PurpleConnectionFlags pc_flags = purple_connection_get_flags(connection);
    pc_flags |= PURPLE_CONNECTION_FLAG_NO_IMAGES;
    pc_flags |= PURPLE_CONNECTION_FLAG_NO_FONTSIZE;
    pc_flags |= PURPLE_CONNECTION_FLAG_NO_BGCOLOR;
    purple_connection_set_flags(connection, pc_flags);
    purple_connection_set_state(connection, PURPLE_CONNECTION_STATE_CONNECTING);
    Presage *presage = g_new0(Presage, 1);
    purple_connection_set_protocol_data(connection, presage);
    const char *user_dir = purple_user_dir();
    const char *username = purple_account_get_username(account);
    const int startup_delay_seconds = purple_account_get_int(account, PRESAGE_STARTUP_DELAY_SECONDS_OPTION, 1);
    char *store_path = g_strdup_printf("%s/presage/%s.db3", user_dir, username);
    presage_rust_main(account, rust_runtime, store_path, startup_delay_seconds); // returns immediately, the actual work happens on the rust runtime's threads
    g_free(store_path);
}

void presage_close(PurpleConnection *connection) {
    PurpleAccount *account = purple_connection_get_account(connection);
    Presage *presage = purple_connection_get_protocol_data(connection);
    presage_rust_exit(account, rust_runtime, presage->tx_ptr);
    presage->tx_ptr = NULL; // presage_rust_exit drops tx, we must no longer use it
}

/*
 * This is a variant of purple_connection_error. It must be run on the main thread.
 *
 * The switch regarding Spectrum is necessary since Spectrum may send commands to the backend 
 * before the backend signals readiness by explicitly setting the account to "connected".
 */
// TODO: Investigate why this happens, when excactly and on which commands. Then narrow down so not all errors are ignored. It is quite possible that the check regarding presage->error solved the issue.
// TODO: Maybe we should never call purple_connection_error_reason here, only log for all UIs?
void presage_account_error(PurpleAccount *account, PurpleConnectionError reason, const char *description) {
    GHashTable *ui_info = purple_core_get_ui_info();
    const gchar *ui_name = g_hash_table_lookup(ui_info, "name");
    if (purple_strequal(ui_name, "Spectrum")) {
        purple_debug_error(PLUGIN_NAME, "Host application is Spectrum. Error is ignored: %s\n", description);
    } else {
        PurpleConnection *connection = purple_account_get_connection(account);
        if (connection != NULL) {
            Presage *presage = purple_connection_get_protocol_data(connection);
            if (presage->error == TRUE) {
                purple_debug_error(PLUGIN_NAME, "Ignoring subsequent error: %s\n", description);
                // an error has alreade been reported, do not report an error again
                // this should be covered by the check for connection->disconnect_timeout > 0 in purple_connection_error_reason,
                // but due to the asynchronous nature of the rust part, errors might come in after the disconnect_timeout has happened
                // The current connection cannot recover from the error state. It will be destroyed and a new connection can be established.
            } else {
                purple_connection_error_reason(connection, reason, description);
            }
        }
    }
}
