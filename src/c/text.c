#include "presage.h"

void presage_handle_text(PurpleConnection *connection, const char *who, const char *group, uint64_t sent, uint64_t timestamp, const char *text) {
    PurpleAccount *account = purple_connection_get_account(connection);
    PurpleMessageFlags flags = 0;
    if (sent) {
        // special handling of messages sent by self incoming from remote for Spectrum
        // send-acknowledgements should be PURPLE_MESSAGE_SEND only (without PURPLE_MESSAGE_REMOTE_SEND)
        // for details, look into purple-whatsmeow
        flags |= PURPLE_MESSAGE_SEND | PURPLE_MESSAGE_REMOTE_SEND;
    } else {
        flags |= PURPLE_MESSAGE_RECV;
    }
    
    if (group == NULL) {
        if (flags & PURPLE_MESSAGE_SEND) {
            // display message sent from own account (other device as well as local echo)
            // cannot use purple_serv_got_im since it sets the flag PURPLE_MESSAGE_RECV
            PurpleConversation *conv = purple_find_conversation_with_account(PURPLE_CONV_TYPE_IM, who, account);
            if (conv == NULL) {
                conv = purple_conversation_new(PURPLE_CONV_TYPE_IM, account, who); // MEMCHECK: caller takes ownership
            }
            purple_conv_im_write(purple_conversation_get_im_data(conv), who, text, flags, timestamp/1000);
        } else {
            serv_got_im(connection, who, text, flags, timestamp/1000);
        }
    } else {
        // TODO
    }
}
