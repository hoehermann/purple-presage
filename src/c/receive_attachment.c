#include "presage.h"

static void xfer_init_fnc(PurpleXfer *xfer) {
    purple_xfer_start(xfer, -1, NULL, 0); // invokes start_fnc
}

static void xfer_start_fnc(PurpleXfer * xfer) {
    // TODO: it would be nice to download just now, in a streaming fashion, not beforehand as a single big blob
    purple_xfer_prpl_ready(xfer); // invokes do_transfer which invokes read_fnc
}

static gssize xfer_read_fnc(guchar **buffer, PurpleXfer * xfer) {
    // entire attachment already is in memory.
    // just forward the pointer to the destination buffer.
    *buffer = xfer->data;
    xfer->data = NULL; // MEMCHECK: not our memory to free any more
    return purple_xfer_get_size(xfer);
}

static void xfer_ack_fnc(PurpleXfer * xfer, const guchar * buffer, size_t bytes_read) {
    // This is called after each time xfer_read_fnc returned a positive value.
    // We only do one read, so the transfer is complete now.
    #if PURPLE_VERSION_CHECK(2,14,10)
    purple_xfer_set_completed(xfer, TRUE);
    #endif
}

static void xfer_release_blob(PurpleXfer * xfer) {
    g_free(xfer->data);
    xfer->data = NULL;
}

static void presage_xfer_announce(PurpleConnection *connection, const char *who, const char *group, const char *filename) {
    // resolve identifier for displaying name
    const char * alias = who; // use the identifier by default
    PurpleBuddy * buddy = purple_find_buddy(purple_connection_get_account(connection), who);
    if (buddy != NULL) {
        alias = purple_buddy_get_contact_alias(buddy);
    }
    char * text = g_strdup_printf("Preparing to store \"%s\" sent by %s...", filename, alias); // MEMCHECK: is released here
    // TODO: Also have human-readable group name here? Theoretically, it should already be in the blist.
    presage_handle_text(connection, who, NULL, group, PURPLE_MESSAGE_SYSTEM, time(NULL)*1000, text);
    g_free(text);
}

void presage_handle_attachment(PurpleConnection *connection, const char *who, const char *group, uint64_t timestamp, void *blob, uint64_t blobsize, const char *filename) {
    g_return_if_fail(connection != NULL);
    PurpleAccount *account = purple_connection_get_account(connection);

    presage_xfer_announce(connection, who, group, filename);
    
    const char *sender = who;
    if (group) {
        sender = group;
    }
    PurpleXfer * xfer = purple_xfer_new(account, PURPLE_XFER_RECEIVE, sender);
    purple_xfer_set_filename(xfer, filename);
    purple_xfer_set_size(xfer, blobsize);
    xfer->data = blob;
    
    purple_xfer_set_init_fnc(xfer, xfer_init_fnc);
    purple_xfer_set_start_fnc(xfer, xfer_start_fnc);
    purple_xfer_set_read_fnc(xfer, xfer_read_fnc);
    purple_xfer_set_ack_fnc(xfer, xfer_ack_fnc);
    
    // be very sure to release the data no matter what
    purple_xfer_set_end_fnc(xfer, xfer_release_blob);
    purple_xfer_set_request_denied_fnc(xfer, xfer_release_blob);
    purple_xfer_set_cancel_recv_fnc(xfer, xfer_release_blob);
    
    purple_xfer_request(xfer);
    // MEMCHECK NOTE: purple_xfer_unref calls purple_xfer_destroy which MAY call purple_xfer_cancel_local if (purple_xfer_get_status(xfer) == PURPLE_XFER_STATUS_STARTED) which calls cancel_recv and cancel_local
}
