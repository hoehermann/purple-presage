// struct taken from presage-cli
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
        uuid: presage::libsignal_service::prelude::Uuid,
        message: String,
    },
}
