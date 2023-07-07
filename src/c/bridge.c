#include "presage.h"

/////////////////////////////////////////////////////////////////////
//                                                                 //
//      WELCOME TO THE LAND OF ABANDONMENT OF TYPE AND SAFETY      //
//                        Wanderer, beware.                        //
//                                                                 //
/////////////////////////////////////////////////////////////////////

/*
 * Whether the given pointer actually refers to an existing account.
 */
static int account_exists(PurpleAccount *account)
{
    int exists = 0;
    // this would be more elegant, but bitlbee does not implement purple_accounts_get_all()
    // see https://github.com/hoehermann/purple-gowhatsapp/issues/102
    // for (GList *iter = purple_accounts_get_all(); iter != NULL && exists == 0; iter = iter->next) {
    //     PurpleAccount * acc = (PurpleAccount *)iter->data;
    //     exists = acc == account;
    // }
    for (GList *iter = purple_connections_get_connecting(); iter != NULL && exists == 0; iter = iter->next) {
        PurpleAccount * acc = purple_connection_get_account(iter->data);
        exists = acc == account;
    }
    for (GList *iter = purple_connections_get_all(); iter != NULL && exists == 0; iter = iter->next) {
        PurpleAccount * acc = purple_connection_get_account(iter->data);
        exists = acc == account;
    }
    return exists;
}

/*
 * Handler for a message received by rust.
 * Called inside of the GTK eventloop.
 * Releases almost all memory allocated by CGO on heap.
 *
 * @return Whether to execute again. Always FALSE.
 */
static gboolean process_message(gpointer data) {
    g_return_val_if_fail(data != NULL, FALSE);
    Presage * message = (Presage *)data;
    purple_debug_info(PLUGIN_NAME, "process_message_bridge called.\n");
    purple_debug_info(PLUGIN_NAME, "message is at %p\n", message);
    purple_debug_info(PLUGIN_NAME, "account is at %p\n", message->account);
    purple_debug_info(PLUGIN_NAME, "tx_ptr is at %p\n", message->tx_ptr);
    purple_debug_info(PLUGIN_NAME, "qrcode is at %p\n", (void *)message->qrcode);
    
/*
    if (gwamsg->msgtype == gowhatsapp_message_type_log) {
        // log messages do not need an active connection
        purple_debug(gwamsg->subtype, GOWHATSAPP_NAME, "%s", gwamsg->text);
        return;
    }
*/
    if (account_exists(message->account) == 0) {
        purple_debug_warning(PLUGIN_NAME, "No account %p. Ignoring message.\n", message->account);
        return FALSE;
    }
    PurpleConnection *connection = purple_account_get_connection(message->account);
    if (connection == NULL) {
        purple_debug_warning(PLUGIN_NAME, "No active connection for account %p. Ignoring message.\n", message->account);
        return FALSE;
    }

    if (message->tx_ptr != NULL) {
        presage_rust_link(rust_runtime, message->tx_ptr, "devicename");
    }
    if (message->qrcode != NULL) {
        purple_debug_info(PLUGIN_NAME, "have qrcode data %s\n", message->qrcode);
        // TODO: deallocate qrcode via rust
    }
    
    return FALSE;
}

#if !(GLIB_CHECK_VERSION(2, 67, 3))
#define g_memdup2 g_memdup
#endif

/*
 * Handler for a message received by rust.
 * Called by go-whatsapp (outside of the GTK eventloop).
 * 
 * Yes, this is indeed neccessary – we checked.
 */
void presage_append_message(const Presage *message_rust) {
    Presage *message_heap = g_memdup2(message_rust, sizeof *message_rust);
    purple_timeout_add(
        0, // schedule for immediate execution
        process_message, // handle message in main thread
        message_heap // data to handle in main thread
    );
}
