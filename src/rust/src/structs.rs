/*
 *  Taken from presage-cli
 */
#[derive(Debug, Clone)]
pub enum Cmd {
    Exit,
    Whoami,
    Send {
        recipient: Recipient,
        message: Option<String>,
        xfer: *mut crate::bridge_structs::PurpleXfer,
    },
    ListGroups,
    GetGroupMembers {
        master_key_bytes: [u8; 32],
    },
    GetProfile {
        uuid: presage::libsignal_service::prelude::Uuid,
    },
}

#[derive(Debug, Clone)]
pub enum Recipient {
    Contact(presage::libsignal_service::prelude::Uuid),
    Group(presage::libsignal_service::zkgroup::GroupMasterKeyBytes),
}
