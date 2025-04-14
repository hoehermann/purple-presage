#include "presage.h"

static gboolean rust_main_finished(gpointer account) {
    PurpleConnection *connection = purple_account_get_connection(account);
    if (PURPLE_CONNECTION_STATE_DISCONNECTED == purple_connection_get_state(connection)) {
        purple_debug_info(PLUGIN_NAME, "rust runtime has finished while not connected.\n");
    } else {
        purple_connection_error(connection, PURPLE_CONNECTION_ERROR_OTHER_ERROR, "rust runtime has finished unexpectedly.");
    }
    return FALSE; // tell the gtk event loop not to schedule calling this function again
}

void presage_rust_main(void *, PurpleAccount *, char *); // this is implemented by the rust part

#ifdef WIN32
#include <windows.h>
static DWORD WINAPI
#else
static void * 
#endif 
rust_main(void* account) {
    // NOTE: This code is not being run on the main thread. Reading from account here is asking for trouble. Yet I am optimistic that the data will not be moved around while we are reading it.
    const char *user_dir = purple_config_dir();
    const char *username = purple_account_get_username(account);
    const int startup_delay_seconds = purple_account_get_int(account, PRESAGE_STARTUP_DELAY_SECONDS_OPTION, 1);
    char *store_path = g_strdup_printf("%s/presage/%s", user_dir, username);
    g_usleep(G_USEC_PER_SEC * startup_delay_seconds); // waiting here for alleviates database locking issues O_o
    presage_rust_main(rust_runtime, account, store_path);
    g_free(store_path);
    purple_timeout_add(500, rust_main_finished, account); // wait half a second before assessing the termination – there might be messages lingering in the rust → C bridge queue
    return 0;
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
    #ifdef WIN32
    HANDLE thread = CreateThread(NULL, 0, rust_main, account, 0, NULL);
    // TODO: detach and handle non-happy path
    #else
    pthread_t presage_thread;
    int err = pthread_create(&presage_thread, NULL, rust_main, (void *)account);
    if (err == 0) {
        // detach thread so it is "free'd" as soon it terminates
        pthread_detach(presage_thread);
    } else {
        gchar *errmsg = g_strdup_printf("Could not create thread for connecting in background: %s", strerror(err));
        purple_connection_error(connection, PURPLE_CONNECTION_ERROR_OTHER_ERROR, errmsg);
        g_free(errmsg);
    }
    #endif
}

void presage_close(PurpleConnection *connection) {
    Presage *presage = purple_connection_get_protocol_data(connection);
    presage_rust_exit(connection, rust_runtime, presage->tx_ptr);
    // TODO: deallocate protocol data (including rust channel)
}

/*
 * This is a variant of purple_connection_error. It must be run on the main thread.
 *
 * The switch regarding Spectrum is necessary since Spectrum may send commands to the backend 
 * before the backend signals readiness by explicitly setting the account to "connected".
 */
// TODO: Investigate why this happens, when excactly and on which commands. Then narrow down so not all errors are ignored.
void presage_account_error(PurpleAccount *account, PurpleConnectionError reason, const char *description) {
    GHashTable *ui_info = purple_core_get_ui_info();
    const gchar *ui_name = g_hash_table_lookup(ui_info, "name");
    if (purple_strequal(ui_name, "Spectrum")) {
        purple_debug_error(PLUGIN_NAME, "Host application is Spectrum. Error is ignored: %s\n", description);
    } else {
        PurpleConnection *connection = purple_account_get_connection(account);
        purple_connection_error_reason(connection, reason, description);
    }
}