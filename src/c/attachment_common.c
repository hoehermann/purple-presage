#include "attachment_common.h"

static void replace_placeholder(gpointer key, gpointer value, gpointer user_data) {
    if (value) {
        char **text = user_data;
        // NOTE: I am not using g_string_replace here since the GLib shipped with win32 Pidgin is ancient
        char *replaced = purple_strreplace(*text, key, value);
        g_free(*text);
        *text = replaced;
    }
}

char * attachment_fill_template(const char *template, GHashTable *replacements, time_t timestamp) {
    // these are the only place-holders which are always available
    // casts necessary to remove const
    g_hash_table_insert(replacements, "$home", (char *)purple_home_dir());
    g_hash_table_insert(replacements, "$purple", (char *)purple_user_dir());
    char *replaced = g_strdup(purple_utf8_strftime(template, localtime(&timestamp)));
    g_hash_table_foreach(replacements, replace_placeholder, &replaced);
    return replaced;
}

#ifndef WIN32
#include <unistd.h> // for symlink

// This feature will not be implemented on win32, since creating directory junctions via reparse-points is insanely cumbersome:
// https://stackoverflow.com/questions/1400549/in-net-how-do-i-create-a-junction-in-ntfs-as-opposed-to-a-symlink

void create_symlinks_recurse(char *path, char *aliased_path) {
    if (strlen(path) <= 1 || strlen(aliased_path) <= 1) {
        // we reached / or . – stop recursion
        return;
    }
    char *parent_directory = g_path_get_dirname(path);
    char *aliased_parent_directory = g_path_get_dirname(aliased_path);
    create_symlinks_recurse(parent_directory, aliased_parent_directory);
    g_free(parent_directory);
    g_free(aliased_parent_directory);
    if (symlink(path, aliased_path) == 0) {
        // TODO: return aliased path, show that in conversation window
    }
}

/*
 * NOTE: This operates on `replacements` destructively.
 */
char * attachment_create_symlinks(PurpleAccount *account, const char *template, GHashTable *replacements, const char *chat_key, const char *buddy_key, time_t timestamp) {
    const char *buddy_alias = g_hash_table_lookup(replacements, buddy_key);
    PurpleBuddy *buddy = purple_find_buddy(account, buddy_alias);
    if (buddy) {
        const char *alias = purple_buddy_get_alias(buddy);
        // do not use alias if it is NULL, empty or containing directory separator (characters unfit for use in file-system are not checked or escaped)
        if (alias != NULL && *alias != 0 && strchr(alias, '/') == NULL) {
            buddy_alias = alias;
        }
    }
    const char *chat_alias = g_hash_table_lookup(replacements, chat_key);
    PurpleChat *chat = purple_blist_find_chat(account, chat_alias);
    if (chat) {
        const char *alias = purple_chat_get_name(chat);
        // do not use alias if it is NULL, empty or containing directory separator (characters unfit for use in file-system are not checked or escaped)
        if (alias != NULL && *alias != 0 && strchr(alias, '/') == NULL) {
            chat_alias = alias;
        }
    }

    // TODO: always store files with their hash, then provide symlink with the filename?
    char *path = attachment_fill_template(template, replacements, timestamp);
    g_hash_table_insert(replacements, (char *)buddy_key, (char *)buddy_alias);
    g_hash_table_insert(replacements, (char *)chat_key, (char *)chat_alias);
    char *aliased_path = attachment_fill_template(template, replacements, timestamp);
    create_symlinks_recurse(path, aliased_path);
    g_free(aliased_path);
    g_free(path);
}
#endif