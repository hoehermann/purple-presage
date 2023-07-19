#include "presage.h"

void presage_handle_text(PurpleConnection *connection, const char *who, const char *group, uint64_t sent, uint64_t timestamp, const char *text) {
    PurpleAccount *account = purple_connection_get_account(connection);
    PurpleMessageFlags flags = 0;
    // taken from purple-whatsmeow
    if (purple_strequal(purple_account_get_username(account), who)) {
        flags |= PURPLE_MESSAGE_SEND;
        // Note: For outgoing messages (no matter if local echo or sent by other device),
        // PURPLE_MESSAGE_SEND must be set due to how purple_conversation_write is implemented
        if (sent) {
            // special handling of messages sent by self incoming from remote for Spectrum
            flags |= PURPLE_MESSAGE_REMOTE_SEND;
        }
    } else {
        flags |= PURPLE_MESSAGE_RECV;
    }
    
    if (group == NULL) {
        serv_got_im(connection, who, text, flags, timestamp/1000);
    } else {
        // TODO
    }
}
