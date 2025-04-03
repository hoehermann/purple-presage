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

void presage_rust_main(void *, PurpleAccount *, char *);

#ifdef WIN32
#include <windows.h>
static DWORD WINAPI
#else
static void * 
#endif 
rust_main(void* account) {
    const char *user_dir = purple_config_dir();
    const char *username = purple_account_get_username(account);
    char *store_path = g_strdup_printf("%s/presage/%s", user_dir, username);
    g_usleep(G_USEC_PER_SEC); // waiting here for a second alleviates database locking issues O_o
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

    /*
    At this point, we assume the account is "connected" immediately. We need to do this since the blist's aliasing functions do not work on disconnected accounts.
    However, the connection is not fully usable, yet. The presage docs at https://github.com/whisperfish/presage/blob/3f55d5f/presage/src/manager/registered.rs#L574 state: 
    „As a client, it is heavily recommended to process incoming messages and wait for the Received::QueueEmpty messages before giving the ability for users to send messages.“
    */
    purple_connection_set_state(connection, PURPLE_CONNECTION_STATE_CONNECTED);
}

void presage_close(PurpleConnection *connection) {
    Presage *presage = purple_connection_get_protocol_data(connection);
    presage_rust_exit(connection, rust_runtime, presage->tx_ptr);
    // TODO: deallocate protocol data (including rust channel)
}
