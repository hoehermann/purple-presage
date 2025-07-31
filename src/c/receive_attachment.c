#include "presage.h"
#include "attachment_common.h"

struct _XferData {
    RustAttachmentPtr attachment_pointer;
    char *who;
    char *group;
    uint64_t timestamp_ms;
};
typedef struct _XferData XferData;

static XferData * xfer_data_new(RustAttachmentPtr attachment_pointer, const char* who, const char *group, uint64_t timestamp_ms) {
    XferData *xfer_data = g_new0(XferData, 1);
    xfer_data->attachment_pointer = attachment_pointer;
    xfer_data->who = g_strdup(who);
    xfer_data->group = g_strdup(group);
    xfer_data->timestamp_ms = timestamp_ms;
    return xfer_data;
}

static void xfer_data_destroy(XferData *xfer_data) {
    g_free(xfer_data->who);
    g_free(xfer_data->group);
    g_free(xfer_data);
}

// This is called after the user accepted the file transfer (and chose a destination)
static void xfer_init(PurpleXfer *xfer) {
    XferData *xfer_data = xfer->data;
    PurpleAccount *account = purple_xfer_get_account(xfer);
    PurpleConnection *connection = purple_account_get_connection(account);
    Presage *presage = purple_connection_get_protocol_data(connection);
    RustAttachmentPtr attachment_pointer = xfer_data->attachment_pointer;
    xfer_data->attachment_pointer = NULL; // the pointer is "consumed" by the rust runtime and must not be released again
    presage_rust_get_attachment(connection, rust_runtime, presage->tx_ptr, attachment_pointer, xfer);
}

static void xfer_release(PurpleXfer * xfer) {
    if (xfer->data != NULL) {
        XferData *xfer_data = xfer->data;
        if (xfer_data->attachment_pointer) {
            presage_rust_drop_attachment(xfer_data->attachment_pointer);
        }
        xfer_data_destroy(xfer_data);
        xfer->data = NULL;
    }
}

static PurpleXfer * xfer_new(PurpleAccount *account, const char *who, const char *group, uint64_t timestamp_ms, const size_t size, const char *filename, RustAttachmentPtr attachment_pointer) {    
    //purple_debug_info(PLUGIN_NAME, "xfer_new(%s, %s, %ld, %lu, %s, %p)…\n", who, group, timestamp_ms, size, filename, attachment_pointer);
    const char *sender = who;
    if (group) {
        sender = group;
    }
    PurpleXfer * xfer = purple_xfer_new(account, PURPLE_XFER_RECEIVE, sender);
    purple_xfer_set_filename(xfer, filename);
    purple_xfer_set_size(xfer, size);
    xfer->data = xfer_data_new(attachment_pointer, who, group, timestamp_ms);
    // NOTE: xfer->message cannot be used for the caption since in purple_xfer_ask_recv message is automatically written to the conversation of the sender, but purple_xfer_ask_recv does not consider the case where the sender is a chat. also purple_xfer_ask_recv disregards the message timestamp
    
    purple_xfer_set_init_fnc(xfer, xfer_init);
    
    // be very sure to release the data no matter what
    purple_xfer_set_end_fnc(xfer, xfer_release);
    purple_xfer_set_request_denied_fnc(xfer, xfer_release);
    purple_xfer_set_cancel_recv_fnc(xfer, xfer_release);
    
    return xfer;
    // MEMCHECK NOTE: purple_xfer_unref calls purple_xfer_destroy which MAY call purple_xfer_cancel_local if (purple_xfer_get_status(xfer) == PURPLE_XFER_STATUS_STARTED) which calls cancel_recv and cancel_local
}

void presage_handle_attachment(PurpleConnection *connection, const char *who, const char *group, uint64_t timestamp, RustAttachmentPtr attachment_pointer, uint64_t size, const char *hash, const char *filename, const char *extension) {
    g_return_if_fail(connection != NULL);
    Presage *presage = purple_connection_get_protocol_data(connection);
    PurpleAccount *account = purple_connection_get_account(connection);
    // local path for auto-downloader
    const char *local_path_template = purple_account_get_string(account, PRESAGE_ATTACHMENT_PATH_TEMPLATE_OPTION, "");
    if (local_path_template && local_path_template[0]) {
        PurpleMessageFlags flags = PURPLE_MESSAGE_RECV; // TODO: actually distinguish direction
        char *local_path = attachment_fill_template(local_path_template, timestamp, hash, filename, extension, group, who, NULL, flags);
        PurpleXfer * xfer = xfer_new(account, who, group, timestamp, size, NULL, NULL);
        purple_xfer_set_local_filename(xfer, local_path); // NOTE: when this is set, purple_xfer_request(xfer) will not ask the user for the file destination
        presage_rust_get_attachment(connection, rust_runtime, presage->tx_ptr, attachment_pointer, xfer);
        g_free(local_path);
    } else {
        char *filename_full = g_strdup_printf("%s%s%s", hash, filename, extension);
        PurpleXfer * xfer = xfer_new(account, who, group, timestamp, size, filename_full, attachment_pointer);
        purple_xfer_request(xfer);
        g_free(filename_full);
    }
}

/*
 * This gets called when a transfer finishes.
 *
 * Conveniently, the base-line mechanism is the same for all xfers regardless of direction (upload/download).
 * 
 * In case of automated downloads, the link to the stored file is written to the respective conversation window.
 */
void presage_handle_xfer(PurpleXfer *xfer, PurpleMessageFlags flags, const char* error) {
    purple_debug_info(PLUGIN_NAME, "presage_handle_xfer(…)…\n");
    PurpleAccount *account = purple_xfer_get_account(xfer);
    if (flags & PURPLE_MESSAGE_ERROR) {
        const char *destination = purple_xfer_get_remote_user(xfer);
        purple_xfer_error(purple_xfer_get_type(xfer), account, destination, error); 
        purple_xfer_cancel_local(xfer);
        //gowhatsapp_display_text_message(gwamsg->account, gwamsg->senderJid, gwamsg->remoteJid, error, gwamsg->timestamp, gwamsg->isGroup, gwamsg->isOutgoing, gwamsg->name, PURPLE_MESSAGE_ERROR, gwamsg->messageId, TRUE);
    } else {
        purple_xfer_set_bytes_sent(xfer, purple_xfer_get_size(xfer));
        purple_xfer_set_completed(xfer, TRUE);
        
        if (purple_xfer_get_type(xfer) == PURPLE_XFER_RECEIVE) {
            const char *local_path_template = purple_account_get_string(account, PRESAGE_ATTACHMENT_PATH_TEMPLATE_OPTION, "");
            if (local_path_template && local_path_template[0]) {
                #ifndef WIN32
                //create_symlinks(account, local_path_template, timestamp, hash, filename, extension, group, who, NULL, flags);
                #endif
                PurpleConnection *connection = purple_account_get_connection(account);
                XferData *xfer_data = xfer->data;
                char *body = g_filename_to_uri(purple_xfer_get_local_filename(xfer), NULL, NULL);;
                presage_handle_text(connection, xfer_data->who, NULL, xfer_data->group, flags, xfer_data->timestamp_ms, body);
                g_free(body);
                /*
                const char *url_template = purple_account_get_string(gwamsg->account, GOWHATSAPP_ATTACHMENT_URL_TEMPLATE_OPTION, GOWHATSAPP_ATTACHMENT_URL_TEMPLATE_DEFAULT);
                char *url = gowhatsapp_go_url_from_local_path(local_path);
                if (url_template && url_template[0]) {
                    url = gowhatsapp_attachment_fill_template(url_template, gwamsg->timestamp, gwamsg->hash_hex, gwamsg->filename, gwamsg->extension, gwamsg->remoteJid, gwamsg->senderJid, gwamsg->messageId, flags);
                }
                gowhatsapp_display_text_message(gwamsg->account, gwamsg->senderJid, gwamsg->remoteJid, url, gwamsg->timestamp, gwamsg->isGroup, gwamsg->isOutgoing, gwamsg->name, 0, gwamsg->messageId, TRUE);
                g_free(url);
                gowhatsapp_display_image_inline(gwamsg, local_path);
                gowhatsapp_display_caption(gwamsg);
                */
            }
        }
    }
}