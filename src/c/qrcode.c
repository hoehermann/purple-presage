#include "presage.h"
#include <qrencode.h>

static void qrcode_hide(PurpleConnection *connection, PurpleRequestFields *fields) {
    // nothing to do.
}

static void qrcode_cancel(PurpleConnection *connection, PurpleRequestFields *fields) {
    purple_connection_error(connection, PURPLE_CONNECTION_ERROR_OTHER_ERROR, "Linking was cancelled.");
}

static void show_qrcode(PurpleConnection *connection, const char *qrstring, gchar* qrimgdata, gsize qrimglen) {
    // Dispalay qrcode for scanning
    PurpleRequestFields* fields = purple_request_fields_new();
    PurpleRequestFieldGroup* group = purple_request_field_group_new(NULL);

    purple_request_fields_add_group(fields, group);
    {
        PurpleRequestField *field = purple_request_field_string_new("qr_string", "QR Code Data", qrstring, FALSE);
        purple_request_field_group_add_field(group, field);
    }
    {
        PurpleRequestField *field = purple_request_field_image_new("qr_code", "QR Code", qrimgdata, qrimglen);
        purple_request_field_group_add_field(group, field);
    }

    PurpleAccount *account = purple_connection_get_account(connection);
    purple_request_fields(
        connection, "Signal Protocol", "Link to master device",
        "In the Signal App, go to \"Preferences\" and \"Linked devices\". Scan the QR code below. Wait for the window to close.", 
        fields,
        "Hide", G_CALLBACK(qrcode_hide), 
        "Cancel", G_CALLBACK(qrcode_cancel),
        purple_request_cpar_from_account(account),
        connection);
}

static void generate_and_show_qrcode(PurpleConnection *connection, const char *data) {
    g_return_if_fail(data != NULL);
    QRcode * qrcode = QRcode_encodeString(data, 0, QR_ECLEVEL_L, QR_MODE_8, 1);
    if (qrcode != NULL) {
        int border = 4;
        int zoom = 6;
        int qrcodewidth = qrcode->width;
        int imgwidth = (border*2+qrcodewidth)*zoom;
        // poor man's PBM encoder
        gchar *head = g_strdup_printf("P1 %d %d ", imgwidth, imgwidth);
        int headlen = strlen(head);
        const gsize qrimglen = headlen+imgwidth*2*imgwidth*2;
        gchar * qrimgdata = g_strndup(head, qrimglen);
        g_free(head);
        gchar *imgptr = qrimgdata+headlen;
        // inspired by printQr in https://github.com/nayuki/QR-Code-generator/blob/master/c/qrcodegen-demo.c
        for (int y = 0; y/zoom < qrcodewidth + border*2; y++) {
            for (int x = 0; x/zoom < qrcodewidth + border*2; x++) {
                int yoffset = y/zoom - border;
                int xoffset = x/zoom - border;
                char pixel = '0';
                if (yoffset >= 0 && yoffset < qrcodewidth && xoffset >= 0 && xoffset < qrcodewidth) {
                    pixel = (qrcode->data[yoffset*qrcodewidth + xoffset] & 1) ? '1' : '0';
                }
                *imgptr++ = pixel;
                *imgptr++ = ' ';
            }
        }
        QRcode_free(qrcode);
        show_qrcode(connection, data, qrimgdata, qrimglen);
        g_free(qrimgdata);
    } else {
        purple_debug_info(PLUGIN_NAME, "qrcodegen failed.\n");
    }
}

void presage_handle_qrcode(PurpleConnection *connection, const char *data) {
    g_return_if_fail(data != NULL);
    if (data[0] == 0) {
        // empty string means "linking has finished"
        purple_request_close_with_handle(connection); // close request displaying the QR code
        Presage *presage = purple_connection_get_protocol_data(connection);
        presage_rust_whoami(rust_runtime, presage->tx_ptr); // now that linking is done, get own uuid
    } else {
        generate_and_show_qrcode(connection, data);
    }
}

void presage_request_qrcode(PurpleConnection *connection) {
    Presage *presage = purple_connection_get_protocol_data(connection);
    const char * device_name = purple_account_get_string(presage->account, "device-name", g_get_host_name());
    presage_rust_link(rust_runtime, presage->tx_ptr, device_name);
}

// TODO: maybe move this into connection.c?
void presage_handle_uuid(PurpleConnection *connection, const char *uuid) {
    g_return_if_fail(uuid != NULL);
    if (uuid[0] == 0) {
        presage_request_qrcode(connection);
    } else {
        PurpleAccount *account = purple_connection_get_account(connection);
        const char *username = purple_account_get_username(account);
        if (purple_strequal(username, uuid)) {
            Presage *presage = purple_connection_get_protocol_data(connection);
            presage->uuid = g_strdup(uuid);
            purple_request_close_with_handle(connection); // close request displaying the QR code
            // TODO: do a Cmd::RequestContactsSync (or ReceivingMode::InitialSync) here, wait for completion, then start mark connection as connected and only then start receiving
            presage_rust_receive(rust_runtime, presage->tx_ptr);
            purple_connection_set_state(connection, PURPLE_CONNECTION_STATE_CONNECTED);
            presage_blist_buddies_all_set_state(account, purple_primitive_get_id_from_type(PURPLE_STATUS_AVAILABLE)); // TODO: make user configurable
        } else {
            char * errmsg = g_strdup_printf("Username for this account must be '%s'.", uuid);
            purple_connection_error(connection, PURPLE_CONNECTION_ERROR_OTHER_ERROR, errmsg);
            g_free(errmsg);
        }
    }
}
