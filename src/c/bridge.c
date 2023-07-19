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
 * Handle a message according to its content.
 */
static void handle_message(Presage * message) {
/*
    if (gwamsg->msgtype == gowhatsapp_message_type_log) {
        // log messages do not need an active connection
        purple_debug(gwamsg->subtype, GOWHATSAPP_NAME, "%s", gwamsg->text);
        return;
    }
*/
    if (account_exists(message->account) == 0) {
        purple_debug_warning(PLUGIN_NAME, "No account %p. Ignoring message.\n", message->account);
        return;
    }
    PurpleConnection *connection = purple_account_get_connection(message->account);
    if (connection == NULL) {
        purple_debug_warning(PLUGIN_NAME, "No active connection for account %p. Ignoring message.\n", message->account);
        return;
    }

    Presage *presage = purple_connection_get_protocol_data(connection);
    if (message->tx_ptr != NULL) {
        presage->tx_ptr = message->tx_ptr; // store tx_ptr for use throughout the connection lifetime
        presage_rust_whoami(rust_runtime, presage->tx_ptr);
    }
    if (message->qrcode != NULL) {
        presage_handle_qrcode(connection, message->qrcode);
    }
    if (message->uuid != NULL) {
        presage_handle_uuid(connection, message->uuid);
    }
    if (message->body != NULL) {
        presage_handle_text(connection, message->who, message->group, message->sent, message->timestamp, message->body);
    }
}

/*
 * Process a message received by rust.
 * Called inside of the GTK eventloop.
 * Releases almost all memory allocated by rust.
 *
 * @return Whether to execute again. Always FALSE.
 */
static gboolean process_message(gpointer data) {
    g_return_val_if_fail(data != NULL, FALSE);
    Presage * message = (Presage *)data;
    purple_debug_info(PLUGIN_NAME, "process_message called.\n");
    purple_debug_info(PLUGIN_NAME, "message is at %p\n", message);
    purple_debug_info(PLUGIN_NAME, "account is at %p\n", message->account);
    purple_debug_info(PLUGIN_NAME, "tx_ptr is at %p\n", message->tx_ptr);
    purple_debug_info(PLUGIN_NAME, "qrcode is at %p\n", (void *)message->qrcode);
    handle_message(message);
    // TODO: deallocate message->qrcode via rust
    g_free(message);
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
