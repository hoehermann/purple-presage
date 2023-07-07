/*
 *   cmake template for a libpurple plugin
 *   Copyright (C) 2023 Hermann Höhne
 *
 *   This program is free software: you can redistribute it and/or modify
 *   it under the terms of the GNU General Public License as published by
 *   the Free Software Foundation, either version 3 of the License, or
 *   (at your option) any later version.
 *
 *   This program is distributed in the hope that it will be useful,
 *   but WITHOUT ANY WARRANTY; without even the implied warranty of
 *   MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
 *   GNU General Public License for more details.
 *
 *   You should have received a copy of the GNU General Public License
 *   along with this program.  If not, see <https://www.gnu.org/licenses/>.
 */

#include <purple.h>
#include <stdint.h> // for intptr_t
#include <unistd.h> // for sleep

// for displaying an externally managed version number
#ifndef PLUGIN_VERSION
#error Must set PLUGIN_VERSION in build system
#endif
// https://github.com/LLNL/lbann/issues/117#issuecomment-334333286
#define MAKE_STR(x) _MAKE_STR(x)
#define _MAKE_STR(x) #x

static void presage_close(PurpleConnection *pc) {
    // this is an example
}

typedef struct {
    PurpleAccount *account;
    intptr_t tx_ptr;
} Presage;

static void * rust_runtime = NULL;

void presage_rust_link(void *, char *);
void presage_rust_main(void *, Presage *);

static void * rust_main(void *presage) {
    presage_rust_main(rust_runtime, presage);
}

static void login(PurpleAccount *account) {
    g_return_if_fail(rust_runtime != NULL);
    PurpleConnection *pc = purple_account_get_connection(account);
    Presage *presage = g_new0(Presage, 1);
    presage->account = account;
    purple_connection_set_protocol_data(pc, presage);
    pthread_t try_connect_thread;
    int err = pthread_create(&try_connect_thread, NULL, rust_main, (void *)presage);
    if (err == 0) {
        // detach thread so it is "free'd" as soon it terminates
        pthread_detach(try_connect_thread);
    } else {
        gchar *errmsg = g_strdup_printf("Could not create thread for connecting in background: %s", strerror(err));
        purple_connection_error_reason(purple_account_get_connection(account), PURPLE_CONNECTION_ERROR_NETWORK_ERROR, errmsg);
        g_free(errmsg);
    }
    
    while (presage->tx_ptr == 0) {
        sleep(1);
    }
    printf("c: tx_ptr is now %p\n", (void *)presage->tx_ptr);
}

static const char * list_icon(PurpleAccount *account, PurpleBuddy *buddy) {
    return "signal";
}

static GList * status_types(PurpleAccount *account) {
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

void * presage_rust_init();
void presage_rust_destroy(void *);

static gboolean libpurple2_plugin_load(PurplePlugin *plugin) {
    if (rust_runtime != NULL) {
        return FALSE;
    }
    rust_runtime = presage_rust_init();
    return TRUE;
}

static gboolean libpurple2_plugin_unload(PurplePlugin *plugin) {
    purple_signals_disconnect_by_handle(plugin);
    if (rust_runtime != NULL) {
        presage_rust_destroy(rust_runtime);
    }
    return TRUE;
}

static PurplePluginProtocolInfo prpl_info = {
    .struct_size = sizeof(PurplePluginProtocolInfo), // must be set for PURPLE_PROTOCOL_PLUGIN_HAS_FUNC to work across versions
    .list_icon = list_icon,
    .status_types = status_types, // this actually needs to exist, else the protocol cannot be set to "online"
    .login = login,
    .close = presage_close,
};

static void plugin_init(PurplePlugin *plugin) {
    //
}

static PurplePluginInfo info = {
    .magic = PURPLE_PLUGIN_MAGIC,
    .major_version = PURPLE_MAJOR_VERSION,
    .minor_version = PURPLE_MINOR_VERSION,
    .type = PURPLE_PLUGIN_PROTOCOL,
    .priority = PURPLE_PRIORITY_DEFAULT,
    .id = "hehoe-purple-presage",
    .name = "Signal (presage)",
    .version = MAKE_STR(PLUGIN_VERSION),
    .summary = "",
    .description = "",
    .author = "Hermann Hoehne <hoehermann@gmx.de>",
    .homepage = "https://github.com/hoehermann/purple-presage",
    .load = libpurple2_plugin_load,
    .unload = libpurple2_plugin_unload,
    .extra_info = &prpl_info,
};

PURPLE_INIT_PLUGIN(presage, plugin_init, info);
