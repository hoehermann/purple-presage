/*
 * This unit is geared to be re-used in purple-presage, purple-whatsmeow and potentially other prpls.
 */

#include <purple.h>

char * attachment_fill_template(const char *template, time_t timestamp, const char *hash, const char *filename, const char *extension, const char *remote, const char *sender, const char *messageid, PurpleMessageFlags flags);