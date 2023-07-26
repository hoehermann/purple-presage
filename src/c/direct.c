#include "presage.h"

int presage_send_im(PurpleConnection *connection, const char *who, const char *message, PurpleMessageFlags flags) {
    Presage *presage = purple_connection_get_protocol_data(connection);
    presage_rust_send(rust_runtime, presage->tx_ptr, who, message);
    return 0; // TODO: have various user-configurable ways of displaying success
}
