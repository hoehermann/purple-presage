#include "presage.h"
#include <qrencode.h>

static void qrcode_done(PurpleConnection *connection, PurpleRequestFields *fields) {
    // nothing to do.
}

static void qrcode_cancel(PurpleConnection *connection, PurpleRequestFields *fields) {
    purple_connection_error_reason(connection, PURPLE_CONNECTION_ERROR_OTHER_ERROR, "Linking was cancelled.");
}

static void show_qrcode(PurpleConnection *connection, gchar* qrimgdata, gsize qrimglen) {
    // Dispalay qrcode for scanning
    PurpleRequestFields* fields = purple_request_fields_new();
    PurpleRequestFieldGroup* group = purple_request_field_group_new(NULL);
    PurpleRequestField* field;

    purple_request_fields_add_group(fields, group);

    field = purple_request_field_image_new(
                "qr_code", "QR code",
                 qrimgdata, qrimglen);
    purple_request_field_group_add_field(group, field);

    PurpleAccount *account = purple_connection_get_account(connection);
    purple_request_fields(
        connection, "Signal Protocol", "Link to master device",
        "For linking this account to a Signal master device, "
        "please scan the QR code below. In the Signal App, "
        "go to \"Preferences\" and \"Linked devices\".", 
        fields,
        "Done", G_CALLBACK(qrcode_done), 
        "Cancel", G_CALLBACK(qrcode_cancel),
        account, 
        purple_account_get_username(account), 
        NULL, 
        connection);
}

void presage_handle_qrcode(PurpleConnection *connection, const char *data) {
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
        show_qrcode(connection, qrimgdata, qrimglen);
        g_free(qrimgdata);
    } else {
        purple_debug_info(PLUGIN_NAME, "qrcodegen failed.\n");
    }
}

void presage_request_qrcode(PurpleConnection *connection) {
    Presage *presage = purple_connection_get_protocol_data(connection);
    const char * device_name = purple_account_get_string(presage->account, "device-name", g_get_host_name());
    presage_rust_link(rust_runtime, presage->tx_ptr, device_name);
}

void presage_handle_uuid(PurpleConnection *connection, const char *uuid) {
    if (uuid[0] == 0) {
        presage_request_qrcode(connection);
    } else {
        const char *username = purple_account_get_username(purple_connection_get_account(connection));
        if (purple_strequal(username, uuid)) {
            Presage *presage = purple_connection_get_protocol_data(connection);
            presage->uuid = g_strdup(uuid);
            purple_request_close_with_handle(connection); // close request displaying the QR code
            purple_connection_set_state(connection, PURPLE_CONNECTED);
            presage_rust_receive(rust_runtime, presage->tx_ptr);
        } else {
            char * errmsg = g_strdup_printf("Your username '%s' does not match the main device's ID '%s'. Please adjust your username.", username, uuid);
            purple_connection_error_reason(connection, PURPLE_CONNECTION_ERROR_OTHER_ERROR, errmsg);
            g_free(errmsg);
        }
    }
}
