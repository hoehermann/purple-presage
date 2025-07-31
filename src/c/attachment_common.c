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

char * attachment_fill_template(const char *template, time_t timestamp, const char *hash, const char *filename, const char *extension, const char *chat, const char *sender, const char *messageid, PurpleMessageFlags flags) {
    // in case of direct conversations, the chat field may be unset
    if (chat == NULL) {
        chat = sender;
    }
    // in case of chats, chat and sender may be different
    // but in case of direct messages, they are the same
    // I do not want the sender to appear twice
    if (purple_strequal(chat, sender)) {
        sender = "";
    }
    const char *direction = "";
    if (flags & PURPLE_MESSAGE_RECV) {
        direction = "received";
    }
    if (flags & PURPLE_MESSAGE_SEND) {
        direction = "sent";
    }

    // this hash table does not release keys since they are static
    // it does not release values since they are not owned by this function
    GHashTable *replacements = g_hash_table_new_full(g_str_hash, g_str_equal, NULL, NULL);
    // casts necessary to remove const
    g_hash_table_insert(replacements, "$home", (char *)purple_home_dir());
    g_hash_table_insert(replacements, "$purple", (char *)purple_user_dir());
    g_hash_table_insert(replacements, "$hash", (char *)hash);
    g_hash_table_insert(replacements, "$direction", (char *)direction);
    g_hash_table_insert(replacements, "$chat", (char *)chat);
    g_hash_table_insert(replacements, "$sender", (char *)sender);
    g_hash_table_insert(replacements, "$messageid", (char *)messageid);
    g_hash_table_insert(replacements, "$extension", (char *)extension);
    g_hash_table_insert(replacements, "$filename", (char *)filename); // NOTE: weird things could happen if the filename contains a placeholder…

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

char * attachment_create_symlinks(PurpleAccount *account, const char *template, time_t timestamp, const char *hash, const char *filename, const char *extension, const char *remote, const char *sender, const char *messageid, PurpleMessageFlags flags) {
    const char *chat_alias = remote;
    const char *buddy_alias = sender;
    PurpleBuddy *buddy = purple_find_buddy(account, sender);
    if (buddy) {
        const char *alias = purple_buddy_get_alias(buddy);
        // do not use alias if it is NULL, empty or containing directory separator (characters unfit for use in file-system are not checked or escaped)
        if (alias != NULL && *alias != 0 && strchr(alias, '/') == NULL) {
            buddy_alias = alias;
        }
    }
    PurpleChat *chat = purple_blist_find_chat(account, remote);
    if (chat) {
        const char *alias = purple_chat_get_name(chat);
        // do not use alias if it is NULL, empty or containing directory separator (characters unfit for use in file-system are not checked or escaped)
        if (alias != NULL && *alias != 0 && strchr(alias, '/') == NULL) {
            chat_alias = alias;
        }
    }
    if (purple_strequal(remote, sender)) {
        // chat is contact (direct message)
        chat_alias = buddy_alias;
    } else {
        // group chat
    }
    // TODO: always store files with their hash, then provide symlink with the filename?
    char *aliased_path = attachment_fill_template(template, timestamp, hash, filename, extension, chat_alias, buddy_alias, messageid, flags);
    char *path = attachment_fill_template(template, timestamp, hash, filename, extension, remote, sender, messageid, flags);
    create_symlinks_recurse(path, aliased_path);
    g_free(aliased_path);
    g_free(path);
}
#endif