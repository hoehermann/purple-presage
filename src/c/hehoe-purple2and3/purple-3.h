#pragma once

// account
#define purple_account_get_username(account) purple_contact_info_get_username(PURPLE_CONTACT_INFO(account))

// connections
#define purple_connections_get_connecting() NULL // TODO

// conversation
#define purple_conversation_find_im_by_name(name, account) purple_conversation_manager_find_im(purple_conversation_manager_get_default(), account, name); // neither
#define purple_conversation_get_im_data(conv) conv
void purple_conv_im_write(PurpleConversation *conv, const char *who, const char *message, PurpleMessageFlags flags, time_t mtime);

// request
#define PurpleRequestFields PurpleRequestPage
#define PurpleRequestFieldGroup PurpleRequestGroup
#define purple_request_fields_new purple_request_page_new
#define purple_request_fields_add_group purple_request_page_add_group
#define purple_request_field_group_new purple_request_group_new
#define purple_request_field_group_add_field purple_request_group_add_field

// timeout
#define purple_timeout_add g_timeout_add
