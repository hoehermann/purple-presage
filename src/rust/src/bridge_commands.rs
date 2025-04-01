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
            println!("presage: rust: command_tx.send {err}");
            // NOTE: this happens whenever the rust mainloop terminates earlier than the purple connection is destroyed
            // TODO: can we call purple_error directly as this is executed in the main glib thread?
        }
    }
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
pub unsafe extern "C" fn presage_rust_send(
    rt: *mut tokio::runtime::Runtime,
    tx: *mut tokio::sync::mpsc::Sender<crate::structs::Cmd>,
    c_destination: *const std::os::raw::c_char,
    c_message: *const std::os::raw::c_char,
    xfer: *mut crate::bridge_structs::PurpleXfer,
) {
    // TODO: add error handling instead of blind unwrap()
    let destination= std::ffi::CStr::from_ptr(c_destination).to_str().unwrap();
    let d = destination.as_bytes();
    let recipient = if d.len() == 36 && d[8] == b'-' && d[13] == b'-' && d[18] == b'-' && d[23] == b'-' {
        // destination looks like a UUID, assume it is a contact
        let uuid = presage::libsignal_service::prelude::Uuid::parse_str(destination).unwrap();
        crate::structs::Recipient::Contact(uuid)
    } else {
        let master_key_bytes = parse_group_master_key(destination);
        crate::structs::Recipient::Group(master_key_bytes)
    };
    let cmd = crate::structs::Cmd::Send {
        recipient: recipient,
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
pub unsafe extern "C" fn presage_rust_get_profile(
    rt: *mut tokio::runtime::Runtime,
    tx: *mut tokio::sync::mpsc::Sender<crate::structs::Cmd>,
    c_uuid: *const std::os::raw::c_char,
) {
    // TODO: add error handling instead of unwrap()
    let uuid = presage::libsignal_service::prelude::Uuid::parse_str(std::ffi::CStr::from_ptr(c_uuid).to_str().unwrap()).unwrap();
    let cmd = crate::structs::Cmd::GetProfile { uuid };
    send_cmd(rt, tx, cmd);
}
