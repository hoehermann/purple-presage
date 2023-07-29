// Cmd Struct taken from presage-cli main
#[derive(Debug)]
pub enum Cmd {
    LinkDevice {
        servers: presage::prelude::SignalServers,
        device_name: String,
    },
    Exit,
    Whoami,
    Receive,
    Send {
        uuid: presage::prelude::Uuid,
        message: String,
    },
}
