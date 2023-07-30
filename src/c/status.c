#include "presage.h"

GList * presage_status_types(PurpleAccount *account) {
    GList *types = NULL;
    {
        PurpleStatusType * status = purple_status_type_new(PURPLE_STATUS_AVAILABLE, NULL, NULL, TRUE);
        types = g_list_append(types, status);
    }
    {
        PurpleStatusType * status = purple_status_type_new(PURPLE_STATUS_OFFLINE, NULL, NULL, TRUE);
        types = g_list_append(types, status);
    }
    return types;
}
