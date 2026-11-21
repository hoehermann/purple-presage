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
        xfer: *const crate::bridge_structs::PurpleXfer,
    },
    ListGroups,
    GetGroupMembers {
        master_key_bytes: [u8; 32],
    },
    GetProfile {
        uuid: presage::libsignal_service::prelude::Uuid,
    },
    GetAttachment {
        attachment_pointer: presage::proto::AttachmentPointer,
        xfer: *const crate::bridge_structs::PurpleXfer,
    },
}

#[derive(Debug, Clone)]
pub enum Recipient {
    Contact(presage::libsignal_service::prelude::Uuid),
    // An E.164 phone number (e.g. "+15555550100"); resolved to a contact UUID in the command loop.
    // Signal buddies are UUID-keyed, but a phone address reaches us from cross-service contact linking.
    ContactByPhone(String),
    Group(presage::libsignal_service::zkgroup::GroupMasterKeyBytes),
}
