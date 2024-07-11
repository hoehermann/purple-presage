/*
 *  Taken from presage-cli
 */
#[derive(Debug, Clone)]
pub enum Cmd {
    LinkDevice {
        servers: presage::libsignal_service::configuration::SignalServers,
        device_name: String,
    },
    Exit,
    Whoami,
    InitialSync,
    Receive,
    Send {
        recipient: Recipient,
        message: String,
    },
    ListGroups,
    GetGroupMembers {
        master_key_bytes: [u8; 32],
    },
}

#[derive(Debug, Clone)]
pub enum Recipient {
    Contact(presage::libsignal_service::prelude::Uuid),
    Group(presage::libsignal_service::zkgroup::GroupMasterKeyBytes),
}
