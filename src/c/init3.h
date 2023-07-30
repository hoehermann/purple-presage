#pragma once

#include "hehoe-purple2and3/purple.h"

#define PRESAGE_TYPE_PROTOCOL (presage_protocol_get_type())
G_DECLARE_FINAL_TYPE(PresageProtocol, presage_protocol, PRESAGE, PROTOCOL, PurpleProtocol)
