#include "purple.h"

void purple_conv_im_write(PurpleConversation *conv, const char *who, const char *content, PurpleMessageFlags flags, time_t timestamp) {
    // from pidgin3/libpurple/protocols/facebook/util.c
    // also from pidgin3/libpurple/purpleconversation.c common_send
    PurpleAccount *account = purple_conversation_get_account(conv);
    PurpleContactInfo *info = PURPLE_CONTACT_INFO(account);
    const gchar * me = purple_contact_info_get_name_for_display(info);
    //const gchar * name = purple_conversation_get_name(conv);
    PurpleMessage * msg = purple_message_new_outgoing(account, me, who, content, flags);
    GDateTime * dt = g_date_time_new_from_unix_local(timestamp);
    purple_message_set_timestamp(msg, dt);
    g_date_time_unref(dt);
    purple_conversation_write_message(conv, msg);
    g_object_unref(G_OBJECT(msg));
}
