/*
 *   purple-presage
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

#include "presage.h"
#include <gmodule.h> // g_module_make_resident - pin the Rust/Tokio .so so libpurple can't dlclose/unmap it

// for displaying an externally managed version number
#ifndef PLUGIN_VERSION
#error Must set PLUGIN_VERSION in build system
#endif

RustRuntimePtr rust_runtime = NULL;

static const char * list_icon(PurpleAccount *account, PurpleBuddy *buddy) {
    return "signal";
}

static gboolean libpurple2_plugin_load(PurplePlugin *plugin) {
    if (rust_runtime != NULL) {
        return FALSE;
    }
    rust_runtime = presage_rust_init();
    // Pin this module resident. It runs Rust Tokio worker threads; libpurple dlcloses plugins during
    // a plugin (re)probe, which unmaps code the Tokio threads execute AND unregisters prpl-hehoe-presage.
    // Once the prpl is gone, the transport's next Util::createPurpleAccount() call hits
    // purple_find_prpl()==NULL -> getProtocolInfo throws an uncaught MojoException -> std::terminate ->
    // abort -> the whole imlibpurpletransport SIGABRT crash-loops (every account down). Every other
    // async prpl (whatsmeow, tdlib) already pins itself resident; presage was the one that didn't.
    if (plugin && plugin->handle) {
        g_module_make_resident((GModule *) plugin->handle);
    }
    // Per-account store teardown: when the transport deletes an account, purple_accounts_delete()
    // emits "account-removed". Hook it (keyed to this plugin's accounts inside the callback) so the
    // orphaned on-disk Signal store is removed. Handle is the plugin, so it is disconnected in unload.
    purple_signal_connect(purple_accounts_get_handle(), "account-removed", plugin,
                          PURPLE_CALLBACK(presage_account_removed_cb), NULL);
    return TRUE;
}

static gboolean libpurple2_plugin_unload(PurplePlugin *plugin) {
    purple_signals_disconnect_by_handle(plugin);
    if (rust_runtime != NULL) {
        presage_rust_destroy(rust_runtime);
    }
    rust_runtime = NULL;
    return TRUE;
}

static PurplePluginProtocolInfo prpl_info = {
    .struct_size = sizeof(PurplePluginProtocolInfo), // must be set for PURPLE_PROTOCOL_PLUGIN_HAS_FUNC to work across versions
    .list_icon = list_icon,
    .options = OPT_PROTO_NO_PASSWORD,
    .status_types = presage_status_types, // this actually needs to exist, else the protocol cannot be set to "online"
    .login = presage_login,
    .close = presage_close,
    // contact related
    .send_im = presage_send_im,
    .add_buddy = presage_add_buddy,
    .tooltip_text = presage_tooltip_text,
    .get_info = presage_get_info,
    // group chat related
    .find_blist_chat = presage_find_blist_chat,
    .set_chat_topic = presage_set_chat_topic,
    .chat_send = presage_send_chat,
    .chat_info = presage_chat_info,
    .join_chat = presage_join_chat,
    .roomlist_get_list = presage_roomlist_get_list,
    // file transfer
    .send_file = presage_send_file,
    #if PURPLE_VERSION_CHECK(2,14,0)
    .chat_send_file = presage_chat_send_file,
    #endif
};

static void plugin_init(PurplePlugin *plugin) {
    prpl_info.protocol_options = presage_add_account_options(prpl_info.protocol_options);
}

static PurplePluginInfo info = {
    .magic = PURPLE_PLUGIN_MAGIC,
    .major_version = PURPLE_MAJOR_VERSION,
    .minor_version = PURPLE_MINOR_VERSION,
    .type = PURPLE_PLUGIN_PROTOCOL,
    .priority = PURPLE_PRIORITY_DEFAULT,
    .id = "prpl-hehoe-presage",
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
