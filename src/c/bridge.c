#include <purple.h>

#define PLUGIN_NAME "presage"

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
 * Basic message processing.
 * Log messages are always processed.
 * Queries Pidgin for a list of all accounts.
 * Ignores message if no appropriate connection exists.
 */
//static void process_message(void * gwamsg) {
/*
    if (gwamsg->msgtype == gowhatsapp_message_type_log) {
        // log messages do not need an active connection
        purple_debug(gwamsg->subtype, GOWHATSAPP_NAME, "%s", gwamsg->text);
        return;
    }
    if (account_exists(gwamsg->account) == 0) {
        purple_debug_warning(PLUGIN_NAME, "No account %p. Ignoring message.\n", gwamsg->account);
        return;
    }
    PurpleConnection *connection = purple_account_get_connection(gwamsg->account);
    if (connection == NULL) {
        purple_debug_warning(PLUGIN_NAME, "No active connection for account %p. Ignoring message.\n", gwamsg->account);
        return;
    }
*/
    //gowhatsapp_process_message(gwamsg);
//}

/*
 * Handler for a message received by go-whatsapp.
 * Called inside of the GTK eventloop.
 * Releases almost all memory allocated by CGO on heap.
 *
 * @return Whether to execute again. Always FALSE.
 */
static gboolean process_message(gpointer data)
{
    purple_debug_warning(PLUGIN_NAME, "process_message_bridge called.\n");
    purple_debug_warning(PLUGIN_NAME, "data is: %s\n", data);

    //presage_rust_link(rust_runtime, "devicename");
    //process_message
    return FALSE;
}

#if !(GLIB_CHECK_VERSION(2, 67, 3))
#define g_memdup2 g_memdup
#endif

/*
 * Handler for a message received by go-whatsapp.
 * Called by go-whatsapp (outside of the GTK eventloop).
 * 
 * Yes, this is indeed neccessary – we checked.
 */
void presage_append_message(const char *str)
{
    //gowhatsapp_message_t *gwamsg_heap = g_memdup2(&gwamsg_go, sizeof gwamsg_go);
    purple_timeout_add(
        0, // schedule for immediate execution
        process_message, // handle message in main thread
        strdup(str) // data to handle in main thread
    );
}
