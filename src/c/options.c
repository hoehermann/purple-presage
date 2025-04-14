#include "presage.h"

const char * PRESAGE_STARTUP_DELAY_SECONDS_OPTION = "startup-delay-seconds";

GList * presage_add_account_options(GList *account_options) {
    PurpleAccountOption *option;
    
    option = purple_account_option_int_new(
        "How many seconds to wait before starting up",
        PRESAGE_STARTUP_DELAY_SECONDS_OPTION,
        1
        );
    account_options = g_list_append(account_options, option);

    return account_options;
}
