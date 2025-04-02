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
    g_free(message->phone_number);
    g_free(message->group);
    g_free(message->body);
    // message->blob is not released here – it must be released by the xfer callback
    if (message->groups) {
        for (int gi = 0; gi < message->groups_length; gi++) {
            g_free(message->groups[gi].key);
            g_free(message->groups[gi].title);
            g_free(message->groups[gi].description);
            for (int mi = 0; mi < message->groups[gi].population; mi++) {
                g_free(message->groups[gi].members[mi]);
            }
        }
    }
    g_free(message->groups);
    g_free(message);
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
        presage_handle_attachment(connection, message->who, message->group, message->timestamp, message->blob, message->blob_length, message->name);
    } else if (message->xfer != NULL) {
        presage_handle_xfer(message->xfer, message->flags, message->body);
    } else if (message->body != NULL) {
        presage_handle_text(connection, message->who, message->name, message->group, message->flags, message->timestamp, message->body);
    } else if (message->groups != NULL) {
        presage_handle_groups(connection, message->groups, message->groups_length);
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
    return FALSE;
}

#if !(GLIB_CHECK_VERSION(2, 67, 3))
#define g_memdup2 g_memdup
#endif

/*
 * Handler for a message received by rust.
 * Called asynchronously by the rust runtime (outside of the GTK eventloop).
 * 
 * Yes, this is indeed neccessary – we checked.
 */
void presage_append_message(const Message *message_rust) {
    printf("(xx:xx:xx) presage: presage_append_message(…)\n");
    fflush(stdout);
    // create a copy of the message struct on the heap so we can pass it into the main thread
    Message *message_heap = g_memdup2(message_rust, sizeof *message_rust); // this also copies all data of primitive type
    // copy all strings to the heap
    message_heap->qrcode = g_strdup(message_rust->qrcode);
    message_heap->uuid = g_strdup(message_rust->uuid);
    message_heap->who = g_strdup(message_rust->who);
    message_heap->name = g_strdup(message_rust->name);
    message_heap->phone_number = g_strdup(message_rust->phone_number);
    printf("(xx:xx:xx) presage: message_rust->group is at %p\n", message_rust->group);
    fflush(stdout);
    message_heap->group = g_strdup(message_rust->group);
    message_heap->body = g_strdup(message_rust->body);
    message_heap->blob = g_memdup2(message_rust->blob, message_rust->blob_length);
    // copy all groups to the heap
    printf("(xx:xx:xx) presage: message_rust->groups is at %p and has %zu.\n", message_rust->groups, message_rust->groups_length);
    fflush(stdout);
    message_heap->groups = g_new(Group, message_rust->groups_length);
    for (int gi = 0; gi < message_heap->groups_length; gi++) {
        // copy all strings to the heap
        message_heap->groups[gi].key = g_strdup(message_rust->groups[gi].key);
        message_heap->groups[gi].title = g_strdup(message_rust->groups[gi].title);
        message_heap->groups[gi].description = g_strdup(message_rust->groups[gi].description);
        message_heap->groups[gi].population = message_rust->groups[gi].population;
        message_heap->groups[gi].members = g_new(char*, message_rust->groups[gi].population);
        printf("(xx:xx:xx) presage: message_rust->groups[%d].members is at %p and has %zu.\n", gi, message_rust->groups[gi].members, message_heap->groups[gi].population);
        fflush(stdout);
        for (int mi = 0; mi < message_heap->groups[gi].population; mi++) {
            message_heap->groups[gi].members[mi] = g_strdup(message_rust->groups[gi].members[mi]);
        }
    }
    purple_timeout_add(
        0, // schedule for immediate execution
        process_message, // handle message in main thread
        message_heap // data to handle in main thread
    );
}
