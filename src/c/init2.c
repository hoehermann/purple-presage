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

static void plugin_init(PurplePlugin *plugin) {
    // nothing to do
}

static PurplePluginProtocolInfo prpl_info = {
    .struct_size = sizeof(PurplePluginProtocolInfo), // must be set for PURPLE_PROTOCOL_PLUGIN_HAS_FUNC to work across versions
    .list_icon = list_icon,
    .options = OPT_PROTO_NO_PASSWORD,
    .status_types = presage_status_types, // this actually needs to exist, else the protocol cannot be set to "online"
    .login = presage_login,
    .close = presage_close,
    .send_im = presage_send_im,
    .add_buddy = presage_add_buddy,
    .set_chat_topic = presage_set_chat_topic,
    .chat_send = presage_send_chat
};

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
