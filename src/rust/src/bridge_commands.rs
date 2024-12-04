/*
 * Feeds a command into the channel c → rust.
 */
unsafe fn send_cmd(
    rt: *mut tokio::runtime::Runtime,
    tx: *mut tokio::sync::mpsc::Sender<crate::structs::Cmd>,
    cmd: crate::structs::Cmd,
) {
    let command_tx = tx.as_ref().unwrap();
    let runtime = rt.as_ref().unwrap();
    match runtime.block_on(command_tx.send(cmd)) {
        Ok(()) => {
            //println!("rust: command_tx.send OK");
        }
        Err(err) => {
            println!("rust: command_tx.send {err}");
            //crate::core::purple_error(account, 0 /* PURPLE_CONNECTION_ERROR_NETWORK_ERROR */ , format!("Error sending command to the rust runtime: {err:?}"));
            // TODO: can we call purple_error directly as this is executed in the main glib thread?
        }
    }
}

#[no_mangle]
pub unsafe extern "C" fn presage_rust_link(
    rt: *mut tokio::runtime::Runtime,
    tx: *mut tokio::sync::mpsc::Sender<crate::structs::Cmd>,
    c_device_name: *const std::os::raw::c_char,
) {
    let device_name: String = std::ffi::CStr::from_ptr(c_device_name).to_str().unwrap().to_owned();
    let server = presage::libsignal_service::configuration::SignalServers::Production;
    //let server = presage::libsignal_service::configuration::SignalServers::Staging;
    let cmd = crate::structs::Cmd::LinkDevice {
        device_name: device_name,
        servers: server,
    };
    send_cmd(rt, tx, cmd);
}

#[no_mangle]
pub unsafe extern "C" fn presage_rust_stop(
    rt: *mut tokio::runtime::Runtime,
    tx: *mut tokio::sync::mpsc::Sender<crate::structs::Cmd>,
) {
    let cmd = crate::structs::Cmd::Exit {};
    send_cmd(rt, tx, cmd);
}

#[no_mangle]
pub unsafe extern "C" fn presage_rust_exit(
    rt: *mut tokio::runtime::Runtime,
    tx: *mut tokio::sync::mpsc::Sender<crate::structs::Cmd>,
) {
    let cmd = crate::structs::Cmd::Exit {};
    send_cmd(rt, tx, cmd);
}

#[no_mangle]
pub unsafe extern "C" fn presage_rust_whoami(
    rt: *mut tokio::runtime::Runtime,
    tx: *mut tokio::sync::mpsc::Sender<crate::structs::Cmd>,
) {
    let cmd = crate::structs::Cmd::Whoami {};
    send_cmd(rt, tx, cmd);
}

#[no_mangle]
pub unsafe extern "C" fn presage_rust_initial_sync(
    rt: *mut tokio::runtime::Runtime,
    tx: *mut tokio::sync::mpsc::Sender<crate::structs::Cmd>,
) {
    let cmd = crate::structs::Cmd::InitialSync {};
    send_cmd(rt, tx, cmd);
}

#[no_mangle]
pub unsafe extern "C" fn presage_rust_receive(
    rt: *mut tokio::runtime::Runtime,
    tx: *mut tokio::sync::mpsc::Sender<crate::structs::Cmd>,
) {
    let cmd = crate::structs::Cmd::Receive {};
    send_cmd(rt, tx, cmd);
}

// TODO: wire this up completely
#[no_mangle]
pub unsafe extern "C" fn presage_rust_list_groups(
    rt: *mut tokio::runtime::Runtime,
    tx: *mut tokio::sync::mpsc::Sender<crate::structs::Cmd>,
) {
    let cmd = crate::structs::Cmd::ListGroups {};
    send_cmd(rt, tx, cmd);
}

#[no_mangle]
pub unsafe extern "C" fn presage_rust_get_group_members(
    rt: *mut tokio::runtime::Runtime,
    tx: *mut tokio::sync::mpsc::Sender<crate::structs::Cmd>,
    c_group: *const std::os::raw::c_char,
) {
    // TODO: add error handling instead of unwrap()
    let master_key_bytes = parse_group_master_key(std::ffi::CStr::from_ptr(c_group).to_str().unwrap());
    let cmd = crate::structs::Cmd::GetGroupMembers {
        master_key_bytes: master_key_bytes,
    };
    send_cmd(rt, tx, cmd);
}

#[no_mangle]
pub unsafe extern "C" fn presage_rust_send_contact(
    rt: *mut tokio::runtime::Runtime,
    tx: *mut tokio::sync::mpsc::Sender<crate::structs::Cmd>,
    c_uuid: *const std::os::raw::c_char,
    c_message: *const std::os::raw::c_char,
    xfer: *const std::os::raw::c_void,
) {
    // TODO: add error handling instead of unwrap()
    let uuid = presage::libsignal_service::prelude::Uuid::parse_str(std::ffi::CStr::from_ptr(c_uuid).to_str().unwrap()).unwrap();
    let cmd = crate::structs::Cmd::Send {
        recipient: crate::structs::Recipient::Contact(uuid),
        message: if c_message != std::ptr::null() {
            Some(std::ffi::CStr::from_ptr(c_message).to_str().unwrap().to_owned())
        } else {
            None
        },
        xfer: xfer,
    };
    send_cmd(rt, tx, cmd);
}

/*
 * Taken from presage-cli
 */
fn parse_group_master_key(value: &str) -> presage::libsignal_service::zkgroup::GroupMasterKeyBytes {
    // TODO: forward error to front-end
    let master_key_bytes = hex::decode(value).expect("unable to decode hex string");
    master_key_bytes.try_into().expect("master key should be 32 bytes long")
}

#[no_mangle]
pub unsafe extern "C" fn presage_rust_send_group(
    rt: *mut tokio::runtime::Runtime,
    tx: *mut tokio::sync::mpsc::Sender<crate::structs::Cmd>,
    c_group: *const std::os::raw::c_char,
    c_message: *const std::os::raw::c_char,
    xfer: *const std::os::raw::c_void,
) {
    // TODO: add error handling instead of using unwrap()
    let master_key_bytes = parse_group_master_key(std::ffi::CStr::from_ptr(c_group).to_str().unwrap());
    let cmd_send = crate::structs::Cmd::Send {
        recipient: crate::structs::Recipient::Group(master_key_bytes),
        message: if c_message != std::ptr::null() {
            Some(std::ffi::CStr::from_ptr(c_message).to_str().unwrap().to_owned())
        } else {
            None
        },
        xfer: xfer,
    };
    send_cmd(rt, tx, cmd_send);
}
