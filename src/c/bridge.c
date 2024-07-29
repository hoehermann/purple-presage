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

void free_message(Presage * message) {
    // release all the memory
    presage_rust_free_string(message->qrcode);
    presage_rust_free_string(message->uuid);
    presage_rust_free_string(message->who);
    presage_rust_free_string(message->name);
    presage_rust_free_string(message->group);
    presage_rust_free_string(message->title);
    presage_rust_free_string(message->body);
    // message->blob is not released here – it must be released by the xfer callback
    // TODO: free message->groups here // presage_rust_strfreev(message->members, message->size);

}

/*
 * Handle a message according to its content.
 */
static void handle_message(Presage * message) {
    //purple_debug_info(PLUGIN_NAME, "handle_message({.account=%p, .qrcode=„%s“, .uuid=„%s“, .who=„%s“, .name=„%s“, .group=„%s“, .title=„%s“, .body=„%s“})\n", message->account, message->qrcode, message->uuid, message->who, message->name, message->group, message->title, message->body);

    if (message->debug >= 0) {
        // log messages do not need an active connection
        purple_debug(message->debug, PLUGIN_NAME, "%s", message->body);
        free_message(message);
        return;
    }
    if (account_exists(message->account) == 0) {
        purple_debug_warning(PLUGIN_NAME, "No account %p. Ignoring message.\n", message->account);
        free_message(message);
        return;
    }
    PurpleConnection *connection = purple_account_get_connection(message->account);
    if (connection == NULL) {
        purple_debug_warning(PLUGIN_NAME, "No active connection for account %p. Ignoring message.\n", message->account);
        free_message(message);
        return;
    }

    Presage *presage = purple_connection_get_protocol_data(connection);
    if (message->tx_ptr != NULL) {
        presage->tx_ptr = message->tx_ptr; // store tx_ptr for use throughout the connection lifetime
        presage_rust_whoami(rust_runtime, presage->tx_ptr);
    } else if (message->qrcode != NULL) {
        presage_handle_qrcode(connection, message->qrcode);
    } else if (message->uuid != NULL) {
        presage_handle_uuid(connection, message->uuid);
    } else if (message->connected >= 0) {
        // backend says, connection has been set-up, start receiving
        // TODO: protect against starting more than one receiver
        presage_rust_receive(rust_runtime, presage->tx_ptr);
        purple_connection_set_state(connection, PURPLE_CONNECTION_STATE_CONNECTED);
        presage_blist_buddies_all_set_online(purple_connection_get_account(connection)); // TODO: make user configurable
    } else if (message->error >= 0) {
        purple_connection_error(connection, message->error, message->body);
    } else if (message->blob != NULL) {
        presage_handle_attachment(connection, message->who, message->timestamp, message->blob, message->size, message->name);
    } else if (message->xfer != NULL) {
        presage_handle_xfer(message->xfer, message->flags, message->body);
    } else if (message->body != NULL) {
        presage_handle_text(connection, message->who, message->name, message->group, message->title, message->flags, message->timestamp, message->body);
    } else if (message->groups != NULL) {
        presage_handle_groups(connection, message->groups, message->size);
    }
    free_message(message);
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
    handle_message(message);
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
