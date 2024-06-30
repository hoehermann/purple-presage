/*
 *  Taken from presage-cli
 */
#[derive(Debug)]
pub enum Cmd {
    LinkDevice {
        servers: presage::libsignal_service::configuration::SignalServers,
        device_name: String,
    },
    Exit,
    Whoami,
    Receive,
    Send {
        recipient: Recipient,
        message: String,
    },
}

#[derive(Debug)]
pub enum Recipient {
    Contact(presage::libsignal_service::prelude::Uuid),
    Group(presage::libsignal_service::zkgroup::GroupMasterKeyBytes),
}
