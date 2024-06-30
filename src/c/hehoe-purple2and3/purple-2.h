#pragma once

#define purple_config_dir() purple_user_dir()

// blist
#define purple_blist_find_buddies purple_find_buddies

// connection
#define purple_connection_error purple_connection_error_reason
#define PURPLE_CONNECTION_FLAG_NO_BGCOLOR PURPLE_CONNECTION_NO_BGCOLOR
#define PURPLE_CONNECTION_FLAG_NO_FONTSIZE PURPLE_CONNECTION_NO_FONTSIZE
#define PURPLE_CONNECTION_FLAG_NO_IMAGES PURPLE_CONNECTION_NO_IMAGES
#define purple_connection_get_flags(pc) ((pc)->flags)
#define purple_connection_set_flags(pc, f) ((pc)->flags = (f))
#define PURPLE_CONNECTION_STATE_CONNECTED PURPLE_CONNECTED
#define PURPLE_CONNECTION_STATE_CONNECTING PURPLE_CONNECTING
#define PURPLE_CONNECTION_STATE_DISCONNECTED PURPLE_DISCONNECTED

// conversation
#define purple_conversation_find_im_by_name(who, account) purple_find_conversation_with_account(PURPLE_CONV_TYPE_IM, who, account) // neither

// im_conversation
#define purple_im_conversation_new(account, from) purple_conversation_new(PURPLE_CONV_TYPE_IM, account, from)

// protocol
#define purple_protocol_got_user_status purple_prpl_got_user_status

// request
#define purple_request_cpar_from_account(account) account, NULL, NULL

// serv
#define purple_serv_got_im serv_got_im
#define purple_serv_got_chat_in serv_got_chat_in