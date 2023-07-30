#include <gplugin.h>
#include <gplugin-native.h>

//#include <purple.h>

/*
 * Protocol functions
 */

static const gchar * presage_protocol_actions_get_prefix(PurpleProtocolActions *actions) {
  return "prpl-presage";
}

static void presage_protocol_login(PurpleProtocol *, PurpleAccount *account) {
  presage_login(account);
}

static void presage_protocol_close(PurpleProtocol *, PurpleConnection *connection) {
  presage_close(connection);
}

static GList * presage_protocol_status_types(PurpleProtocol *, PurpleAccount *account) {
  return presage_status_types(account);
}

static void presage_protocol_add_buddy(PurpleProtocolServer *protocol_server, PurpleConnection *connection, PurpleBuddy *buddy, PurpleGroup *group, const gchar *message) {
  presage_add_buddy(connection, buddy, group);
}

int presage_protocol_send_im(PurpleProtocolIM *im, PurpleConnection *gc, PurpleMessage *msg) {
    const gchar * who = purple_message_get_recipient(msg);
    const gchar * message = purple_message_get_contents(msg);
    PurpleMessageFlags flags = purple_message_get_flags(msg);
    return presage_send_im(gc, who, message, flags);
}

/*
 * Initialize the protocol instance. See protocol.h for more information.
 */
static void presage_protocol_init(PresageProtocol *self) {
}

/*
 * Initialize the protocol class and interfaces.
 * See protocol.h for more information.
 */
static void presage_protocol_class_init(PresageProtocolClass *klass) {
  PurpleProtocolClass *protocol_class = PURPLE_PROTOCOL_CLASS(klass);

  protocol_class->login = presage_protocol_login;
  protocol_class->close = presage_protocol_close;
  protocol_class->status_types = presage_protocol_status_types;

  protocol_class->get_account_options = presage_protocol_get_account_options;
  //protocol_class->get_user_splits = presage_protocol_get_user_splits;
}

static void presage_protocol_class_finalize(G_GNUC_UNUSED PresageProtocolClass *klass) {
}

static void presage_protocol_actions_iface_init(PurpleProtocolActionsInterface *iface) {
  iface->get_prefix = presage_protocol_actions_get_prefix;
  iface->get_action_group = presage_protocol_actions_get_action_group;
  iface->get_menu = presage_protocol_actions_get_menu;
}

static void presage_protocol_client_iface_init(PurpleProtocolClientInterface *client_iface) {
  //client_iface->status_text     = presage_status_text;
  //client_iface->tooltip_text    = presage_protocol_tooltip_text;
  //client_iface->blist_node_menu = presage_protocol_blist_node_menu;
}

static void presage_protocol_server_iface_init(PurpleProtocolServerInterface *server_iface) {
  //server_iface->register_user  = presage_register_user; // this makes a "register on server" option appear
  //server_iface->get_info       = presage_protocol_get_info;
  //server_iface->set_status     = presage_set_status;
  server_iface->add_buddy      = presage_protocol_add_buddy;
  //server_iface->alias_buddy    = presage_alias_buddy;
  //server_iface->remove_group   = presage_remove_group;
}

static void presage_protocol_im_iface_init(PurpleProtocolIMInterface *im_iface) {
  im_iface->send        = presage_protocol_send_im;
  //im_iface->send_typing = presage_send_typing;
}

static void presage_protocol_chat_iface_init(PurpleProtocolChatInterface *chat_iface) {
  chat_iface->info          = presage_protocol_chat_info;
  chat_iface->info_defaults = presage_protocol_chat_info_defaults;
  chat_iface->join          = presage_protocol_join_chat;
  //chat_iface->reject        = presage_reject_chat;
  chat_iface->get_name      = presage_protocol_get_chat_name;
  //chat_iface->invite        = presage_chat_invite;
  //chat_iface->leave         = presage_protocol_chat_leave;
  chat_iface->send          = presage_protocol_chat_send;
  chat_iface->set_topic     = presage_protocol_set_chat_topic;
}

static void
presage_protocol_roomlist_iface_init(PurpleProtocolRoomlistInterface *roomlist_iface) {
  roomlist_iface->get_list        = presage_protocol_roomlist_get_list;
}

/*
 * define the signald protocol type. this macro defines
 * presage_protocol_register_type(PurplePlugin *) which is called in plugin_load()
 * to register this type with the type system, and presage_protocol_get_type()
 * which returns the registered GType.
 */
G_DEFINE_DYNAMIC_TYPE_EXTENDED(
    PresageProtocol, presage_protocol, PURPLE_TYPE_PROTOCOL, 0,
        //G_IMPLEMENT_INTERFACE_DYNAMIC(PURPLE_TYPE_PROTOCOL_ACTIONS, presage_protocol_actions_iface_init)
        //G_IMPLEMENT_INTERFACE_DYNAMIC(PURPLE_TYPE_PROTOCOL_CLIENT, presage_protocol_client_iface_init)
        G_IMPLEMENT_INTERFACE_DYNAMIC(PURPLE_TYPE_PROTOCOL_SERVER, presage_protocol_server_iface_init)
        G_IMPLEMENT_INTERFACE_DYNAMIC(PURPLE_TYPE_PROTOCOL_IM, presage_protocol_im_iface_init)
        //G_IMPLEMENT_INTERFACE_DYNAMIC(PURPLE_TYPE_PROTOCOL_CHAT, presage_protocol_chat_iface_init)
        //G_IMPLEMENT_INTERFACE_DYNAMIC(PURPLE_TYPE_PROTOCOL_ROOMLIST, presage_protocol_roomlist_iface_init)
    );

static PurpleProtocol * presage_protocol_new(void) {
  return PURPLE_PROTOCOL(g_object_new(
    SIGNALD_TYPE_PROTOCOL,
    "id", "prpl-presage",
    "name", "presage",
    "options", OPT_PROTO_NO_PASSWORD | OPT_PROTO_CHAT_TOPIC,
    NULL));
}

static GPluginPluginInfo * presage_query(GError **error) {
  const gchar *authors[] = {
    "Hermann Höhne <hoehermann@gmx.de>",
    NULL
  };
  return purple_plugin_info_new(
    "id",           "prpl-presage",
    "name",         "Presage Signal Protocol",
    "authors",      authors,
    "version",      SIGNALD_PLUGIN_VERSION,
    "category",     "Protocol",
    "summary",      "Protocol plug-in for connecting to Signal via presage",
    //"description",  "Presage Plugin description",
    "website",      "https://github.com/hoehermann/purple-presage",
    "abi-version",  PURPLE_ABI_VERSION,

    /* This third-party plug-in should not use this flags,
     * but without them the plug-in will not be loaded in time.
     */
    //"flags", PURPLE_PLUGIN_INFO_FLAGS_AUTO_LOAD,
    NULL
  );
}

/*
 * Reference to the protocol instance, used for registering signals, prefs,
 * etc. it is set when the protocol is added in plugin_load and is required
 * for removing the protocol in plugin_unload.
 */
static PurpleProtocol *presage_protocol = NULL;

static gboolean presage_load(GPluginPlugin *plugin, GError **error) {
  PurpleProtocolManager *manager = purple_protocol_manager_get_default();
  /* Register the PRESAGE_TYPE_PROTOCOL type in the type system. 
   * This function is defined by G_DEFINE_DYNAMIC_TYPE_EXTENDED. 
   */
  presage_protocol_register_type(G_TYPE_MODULE(plugin));
  /* Add the protocol to the core. */
  presage_protocol = presage_protocol_new();
  if(!purple_protocol_manager_register(manager, presage_protocol, error)) {
    g_clear_object(&presage_protocol);
    return FALSE;
  }
  return TRUE;
}

static gboolean presage_unload(GPluginPlugin *plugin, gboolean shutdown, GError **error) {
  PurpleProtocolManager *manager = purple_protocol_manager_get_default();
  /* Remove the protocol from the core.*/
  if(!purple_protocol_manager_unregister(manager, presage_protocol, error)) {
    return FALSE;
  }
  g_clear_object(&presage_protocol);
  return TRUE;
}

GPLUGIN_NATIVE_PLUGIN_DECLARE(presage);
