#include "presage.h"

void presage_rust_main(void *, PurpleAccount *);

#ifdef WIN32
#include <windows.h>
DWORD WINAPI rust_main(void* data) {
    presage_rust_main(rust_runtime, data);
    printf("presage_rust_main has finished.");
    return 0;
}
#else
static void * rust_main(void *account) {
    presage_rust_main(rust_runtime, account);
    printf("presage_rust_main has finished.");
}
#endif

void presage_login(PurpleAccount *account) {
    purple_debug_info(PLUGIN_NAME, "login for account: %p\n", account);
    g_return_if_fail(rust_runtime != NULL);
    purple_debug_info(PLUGIN_NAME, "rust_runtime is at %p\n", rust_runtime);
    PurpleConnection *pc = purple_account_get_connection(account);
    Presage *presage = g_new0(Presage, 1);
    presage->account = account;
    purple_connection_set_protocol_data(pc, presage);
    #ifdef WIN32
    HANDLE thread = CreateThread(NULL, 0, rust_main, account, 0, NULL);
    #else
    pthread_t presage_thread;
    int err = pthread_create(&presage_thread, NULL, rust_main, (void *)account);
    if (err == 0) {
        // detach thread so it is "free'd" as soon it terminates
        pthread_detach(presage_thread);
    } else {
        gchar *errmsg = g_strdup_printf("Could not create thread for connecting in background: %s", strerror(err));
        purple_connection_error_reason(purple_account_get_connection(account), PURPLE_CONNECTION_ERROR_NETWORK_ERROR, errmsg);
        g_free(errmsg);
    }
    #endif
}

void presage_close(PurpleConnection *pc) {
    // TODO: close connection, stop rust thread, deallocate channel
}
