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

void free_message(Message * message) {
    // release all the memory
    g_free(message->qrcode);
    g_free(message->uuid);
    g_free(message->who);
    g_free(message->name);
    if (message->group) {
        for (int i = 0; i < message->size; i++) {
            // TODO: release fields in groups
        }
    }
    g_free(message->group);
    g_free(message->body);
    // message->blob is not released here – it must be released by the xfer callback
}

/*
 * Handle a message according to its content.
 */
static void handle_message(Message * message) {
    //purple_debug_info(PLUGIN_NAME, "handle_message({.account=%p, .qrcode=„%s“, .uuid=„%s“, .who=„%s“, .name=„%s“, .phone_number=„%s“, .group=„%s“, .flags=0x%x, .body=„%s“})\n", 
    //message->account, message->qrcode, message->uuid, message->who, message->name, message->phone_number, message->group, message->flags, message->body);

    if (message->debug != -1) {
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
        presage_rust_whoami(connection, rust_runtime, presage->tx_ptr);
    } else if (message->qrcode != NULL) {
        presage_handle_qrcode(connection, message->qrcode);
    } else if (message->uuid != NULL) {
        presage_handle_uuid(connection, message->uuid);
    } else if (message->connected > 0) {
        // backend says, connection has been set-up
        purple_connection_set_state(connection, PURPLE_CONNECTION_STATE_CONNECTED);
        presage_blist_buddies_all_set_online(purple_connection_get_account(connection)); // TODO: make user configurable
    } else if (message->error != -1) {
        purple_connection_error(connection, message->error, message->body);
    } else if (message->blob != NULL) {
        presage_handle_attachment(connection, message->who, message->group, message->timestamp, message->blob, message->size, message->name);
    } else if (message->xfer != NULL) {
        presage_handle_xfer(message->xfer, message->flags, message->body);
    } else if (message->body != NULL) {
        presage_handle_text(connection, message->who, message->name, message->group, message->flags, message->timestamp, message->body);
    } else if (message->groups != NULL) {
        presage_handle_groups(connection, message->groups, message->size);
    } else if (message->who) {
        presage_handle_contact(connection, message->who, message->name, message->phone_number);
        presage_show_info(connection, message->who, message->name, message->phone_number);
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
    Message * message = (Message *)data;
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
void presage_append_message(const Message *message_rust) {
    Message *message_heap = g_memdup2(message_rust, sizeof *message_rust);
    // TODO: memdup all fields recursively
    purple_timeout_add(
        0, // schedule for immediate execution
        process_message, // handle message in main thread
        message_heap // data to handle in main thread
    );
}
