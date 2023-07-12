#include "presage.h"

GList * presage_add_account_options(GList *account_options) {
    PurpleAccountOption *option;

    option = purple_account_option_string_new(
                "Name of the device for linking",
                "device-name",
                g_get_host_name() // strdup happens internally
                );
    account_options = g_list_append(account_options, option);
    
    return account_options;
}
