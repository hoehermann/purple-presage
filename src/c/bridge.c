#include "presage.h"
#include <inttypes.h> // for fprinting the 6bit timestamp without warnings on both 64 bit and 32 bit targets

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
    g_free(message->extension);
    g_free(message->filename);
    g_free(message->hash);
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
    /*purple_debug_info(PLUGIN_NAME, 
        "handle_message({"
        ".account=%p, "
        ".qrcode=\"%s\", "
        ".uuid=\"%s\", "
        ".debug=%d, "
        ".error=%d, "
        ".connected=%d, "
        ".attachment_size=%u, "
        ".timestamp=%lu, "
        ".flags=0x%x, "
        ".who=\"%s\", "
        ".name=\"%s\", "
        ".phone_number=\"%s\", "
        ".group=\"%s\", "
        ".body=\"%s\", "
        ".attachment_pointer_box=%p, "
        ".hash=\"%s\", "
        ".filename=\"%s\", "
        ".extension=\"%s\", "
        ".mimetype=\"%s\", "
        ".groups=%p, "
        ".groups_length=%zu, "
        ".xfer=%p"
        "})\n", 
        message->account,
        message->qrcode,
        message->uuid,
        message->debug,
        message->error,
        message->connected,
        message->attachment_size,
        message->timestamp,
        message->flags,
        message->who,
        message->name,
        message->phone_number,
        message->group,
        message->body,
        message->attachment_pointer_box,
        message->hash,
        message->filename,
        message->extension,
        message->mimetype,
        message->groups,
        message->groups_length,
        message->xfer
    );*/
printf("handle_message({\n");
printf("  .account=%p,\n", message->account);
printf("  .qrcode=\"%s\",\n", message->qrcode);
printf("  .uuid=\"%s\",\n", message->uuid);
printf("  .debug=%d,\n", message->debug);
printf("  .error=%d,\n", message->error);
printf("  .connected=%d,\n", message->connected);
printf("  .attachment_size=%u,\n", message->attachment_size);
printf("  .timestamp=%"PRIu64",\n", message->timestamp);
printf("  .flags=0x%x,\n", message->flags);
printf("  .who=\"%s\",\n", message->who);
printf("  .name=\"%s\",\n", message->name);
printf("  .phone_number=\"%s\",\n", message->phone_number);
printf("  .group=\"%s\",\n", message->group);
printf("  .body=\"%s\",\n", message->body);
printf("  .attachment_pointer_box=%p,\n", message->attachment_pointer_box);
printf("  .hash=\"%s\",\n", message->hash);
printf("  .filename=\"%s\",\n", message->filename);
printf("  .extension=\"%s\",\n", message->extension);
printf("  .mimetype=\"%s\",\n", message->mimetype);
printf("  .groups=%p,\n", message->groups);
printf("  .groups_length=%zu,\n", message->groups_length);
printf("  .xfer=%p\n", message->xfer);
printf("})\n");
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
        purple_debug_info(PLUGIN_NAME, "calling presage_rust_whoami()...\n");
        presage_rust_whoami(message->account, rust_runtime, presage->tx_ptr);
    } else if (message->qrcode != NULL) {
        purple_debug_info(PLUGIN_NAME, "calling presage_handle_qrcode()...\n");
        presage_handle_qrcode(connection, message->qrcode);
    } else if (message->uuid != NULL) {
        purple_debug_info(PLUGIN_NAME, "calling presage_handle_uuid()...\n");
        presage_handle_uuid(connection, message->uuid);
    } else if (message->connected > 0) {
        // backend says, connection has been set-up
        purple_connection_set_state(connection, PURPLE_CONNECTION_STATE_CONNECTED);
        presage_blist_buddies_all_set_online(purple_connection_get_account(connection)); // TODO: make user configurable
    } else if (message->error != -1) {
        // NOTE: also take a look at presage_account_error(…)
        if (message->who) {
            // an error related to a specific contact – probably due to a failed profile look-up
            presage_show_info(connection, message->who, message->name, message->phone_number, message->body);
        } else {
            // an error not related to a specific contact – affects the entire connection
            if (presage->error == FALSE) {
                // only handle the first error since that one is significant
                // later, subsequent errors can override the original error message in the UI
                presage->error = TRUE;
                purple_connection_error(connection, message->error, message->body);
            }
        }
    } else if (message->attachment_pointer_box != NULL) {
        purple_debug_info(PLUGIN_NAME, "calling presage_handle_attachment()...\n");
        presage_handle_attachment(connection, message->who, message->group, message->flags, message->timestamp, message->attachment_pointer_box, message->attachment_size, message->hash, message->filename, message->extension);
    } else if (message->xfer != NULL) {
        purple_debug_info(PLUGIN_NAME, "calling presage_handle_xfer_end()...\n");
        presage_handle_xfer_end(message->xfer, message->flags, message->body, message->mimetype);
    } else if (message->body != NULL) {
        purple_debug_info(PLUGIN_NAME, "calling presage_handle_text()...\n");
        presage_handle_text(connection, message->who, message->name, message->group, message->flags, message->timestamp, message->body);
    } else if (message->groups != NULL) {
        purple_debug_info(PLUGIN_NAME, "calling presage_handle_groups()...\n");
        presage_handle_groups(connection, message->groups, message->groups_length);
    } else if (message->who) {
        purple_debug_info(PLUGIN_NAME, "calling presage_handle_contact()...\n");
        presage_handle_contact(connection, message->who, message->name, message->phone_number);
        presage_show_info(connection, message->who, message->name, message->phone_number, NULL);
    }
    free_message(message);
}

/////////////////////////////////////////////////////////////////////
//                                                                 //
//      WELCOME TO THE LAND OF ABANDONMENT OF TYPE AND SAFETY      //
//                        Wanderer, beware.                        //
//                                                                 //
/////////////////////////////////////////////////////////////////////

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
    // create a copy of the message struct on the heap so we can pass it into the main thread
    Message *message_heap = g_memdup2(message_rust, sizeof *message_rust); // this also copies all data of fields with primitive types
    // copy all strings to the heap
    message_heap->qrcode = g_strdup(message_rust->qrcode);
    message_heap->uuid = g_strdup(message_rust->uuid);
    message_heap->who = g_strdup(message_rust->who);
    message_heap->name = g_strdup(message_rust->name);
    message_heap->phone_number = g_strdup(message_rust->phone_number);
    message_heap->group = g_strdup(message_rust->group);
    message_heap->body = g_strdup(message_rust->body);
    message_heap->hash = g_strdup(message_rust->hash);
    message_heap->filename = g_strdup(message_rust->filename);
    message_heap->extension = g_strdup(message_rust->extension);
    message_heap->mimetype = g_strdup(message_rust->mimetype);
    // copy all groups to the heap
    message_heap->groups = g_new(Group, message_rust->groups_length);
    for (int gi = 0; gi < message_heap->groups_length; gi++) {
        // copy all strings to the heap
        message_heap->groups[gi].key = g_strdup(message_rust->groups[gi].key);
        message_heap->groups[gi].title = g_strdup(message_rust->groups[gi].title);
        message_heap->groups[gi].description = g_strdup(message_rust->groups[gi].description);
        message_heap->groups[gi].population = message_rust->groups[gi].population;
        message_heap->groups[gi].members = g_new(char*, message_rust->groups[gi].population);
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
