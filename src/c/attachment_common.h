/*
 * This unit is geared to be re-used in purple-presage, purple-whatsmeow and potentially other prpls.
 */

#include <purple.h>

char * attachment_fill_template(const char *template, GHashTable *replacements, time_t timestamp);