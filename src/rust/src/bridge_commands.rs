extern "C" {
    // this is implemented by libpurple's connection.c
    // TODO: automatically generate declaration from connection.h
    fn purple_connection_error_reason(
        connection: *mut ::std::os::raw::c_void,
        reason: crate::bridge_structs::PurpleConnectionError,
        description: *const ::std::os::raw::c_char,
    );
}

unsafe fn send_cmd(
    purple_connection: *mut ::std::os::raw::c_void,
    rt: *mut tokio::runtime::Runtime,
    tx: *mut tokio::sync::mpsc::Sender<crate::structs::Cmd>,
    cmd: crate::structs::Cmd,
) {
    if let Err(err) = send_cmd_impl(rt, tx, cmd) {
        let errmsg = std::ffi::CString::new(format!("send cmd error: {err}")).unwrap();
        // If the receiver is disconnected in crate::core::mainloop, a network error is reported.
        // The main loop terminates, implicitly closing the command channel immediately.
        // On the next gtk event loop iteration, Pidgin tries to close the connection gracefully, but fails to send the exit command due to the closed channel.
        // Consequently, the error reported here is reported as a transient network error since it can overwrite the previous error (see #18).
        // Looking at libpurple/connection.c, purple_connection_error_reason should not overwrite a previous error, but here we are.
        // TODO: Investigate further. Maybe use separate error reasons based on the command that was going to be sent? Or do not report an error at all, just log?
        purple_connection_error_reason(purple_connection, crate::bridge_structs::PURPLE_CONNECTION_ERROR_NETWORK_ERROR, errmsg.as_ptr());
    }
}

/*
 * Feeds a command into the channel c → rust.
 */
unsafe fn send_cmd_impl(
    rt: *mut tokio::runtime::Runtime,
    tx: *mut tokio::sync::mpsc::Sender<crate::structs::Cmd>,
    cmd: crate::structs::Cmd,
) -> Result<(), anyhow::Error> {
    let command_tx = tx.as_ref().ok_or(anyhow::anyhow!("send_cmd: command channel is missing"))?;
    let runtime = rt.as_ref().ok_or(anyhow::anyhow!("send_cmd: runtime is missing"))?;
    runtime.block_on(command_tx.send(cmd)).map_err(|err| anyhow::anyhow!(err.to_string()))
}

#[no_mangle]
pub unsafe extern "C" fn presage_rust_exit(
    purple_connection: *mut ::std::os::raw::c_void,
    rt: *mut tokio::runtime::Runtime,
    tx: *mut tokio::sync::mpsc::Sender<crate::structs::Cmd>,
) {
    let cmd = crate::structs::Cmd::Exit {};
    send_cmd(purple_connection, rt, tx, cmd);
}

#[no_mangle]
pub unsafe extern "C" fn presage_rust_whoami(
    purple_connection: *mut ::std::os::raw::c_void,
    rt: *mut tokio::runtime::Runtime,
    tx: *mut tokio::sync::mpsc::Sender<crate::structs::Cmd>,
) {
    let cmd = crate::structs::Cmd::Whoami {};
    send_cmd(purple_connection, rt, tx, cmd);
}

// TODO: wire this up completely
#[no_mangle]
pub unsafe extern "C" fn presage_rust_list_groups(
    purple_connection: *mut ::std::os::raw::c_void,
    rt: *mut tokio::runtime::Runtime,
    tx: *mut tokio::sync::mpsc::Sender<crate::structs::Cmd>,
) {
    let cmd = crate::structs::Cmd::ListGroups {};
    send_cmd(purple_connection, rt, tx, cmd);
}

#[no_mangle]
pub unsafe extern "C" fn presage_rust_get_group_members(
    purple_connection: *mut ::std::os::raw::c_void,
    rt: *mut tokio::runtime::Runtime,
    tx: *mut tokio::sync::mpsc::Sender<crate::structs::Cmd>,
    c_group: *const std::os::raw::c_char,
) {
    let master_key_bytes = parse_group_master_key(std::ffi::CStr::from_ptr(c_group).to_str().unwrap());
    match master_key_bytes {
        Ok(master_key_bytes) => {
            let cmd = crate::structs::Cmd::GetGroupMembers {
                master_key_bytes: master_key_bytes,
            };
            send_cmd(purple_connection, rt, tx, cmd);
        }
        Err(err) => {
            let c_errmsg = std::ffi::CString::new(err.to_string()).unwrap();
            purple_connection_error_reason(purple_connection, crate::bridge_structs::PURPLE_CONNECTION_ERROR_OTHER_ERROR, c_errmsg.as_ptr());
        }
    }
}

#[no_mangle]
pub unsafe extern "C" fn presage_rust_send(
    purple_connection: *mut ::std::os::raw::c_void,
    rt: *mut tokio::runtime::Runtime,
    tx: *mut tokio::sync::mpsc::Sender<crate::structs::Cmd>,
    c_destination: *const std::os::raw::c_char,
    c_message: *const std::os::raw::c_char,
    xfer: *mut crate::bridge_structs::PurpleXfer,
) {
    let destination = std::ffi::CStr::from_ptr(c_destination).to_str().unwrap();
    let d = destination.as_bytes();
    let recipient = if d.len() == 36 && d[8] == b'-' && d[13] == b'-' && d[18] == b'-' && d[23] == b'-' {
        // destination looks like a UUID, assume it is a contact
        presage::libsignal_service::prelude::Uuid::parse_str(destination)
            .map(|uuid| crate::structs::Recipient::Contact(uuid))
            .map_err(|err| anyhow::anyhow!(err))
    } else {
        parse_group_master_key(destination).map(|master_key_bytes| crate::structs::Recipient::Group(master_key_bytes))
    };
    match recipient {
        Ok(recipient) => {
            let cmd = crate::structs::Cmd::Send {
                recipient: recipient,
                message: if c_message != std::ptr::null() {
                    Some(std::ffi::CStr::from_ptr(c_message).to_str().unwrap().to_owned())
                } else {
                    None
                },
                xfer: xfer,
            };
            send_cmd(purple_connection, rt, tx, cmd);
        }
        Err(err) => {
            let c_errmsg = std::ffi::CString::new(err.to_string()).unwrap();
            purple_connection_error_reason(purple_connection, crate::bridge_structs::PURPLE_CONNECTION_ERROR_OTHER_ERROR, c_errmsg.as_ptr());
        }
    }
}

/*
 * Taken from presage-cli
 */
fn parse_group_master_key(value: &str) -> Result<presage::libsignal_service::zkgroup::GroupMasterKeyBytes, anyhow::Error> {
    let master_key_bytes = hex::decode(value)?;
    presage::libsignal_service::zkgroup::GroupMasterKeyBytes::try_from(master_key_bytes).map_err(|vec| anyhow::anyhow!("Unable to convert group master key {vec:?}."))
}

#[no_mangle]
pub unsafe extern "C" fn presage_rust_get_profile(
    purple_connection: *mut ::std::os::raw::c_void,
    rt: *mut tokio::runtime::Runtime,
    tx: *mut tokio::sync::mpsc::Sender<crate::structs::Cmd>,
    c_uuid: *const std::os::raw::c_char,
) {
    match presage::libsignal_service::prelude::Uuid::parse_str(std::ffi::CStr::from_ptr(c_uuid).to_str().unwrap()) {
        Ok(uuid) => {
            let cmd = crate::structs::Cmd::GetProfile { uuid };
            send_cmd(purple_connection, rt, tx, cmd);
        }
        Err(err) => {
            let c_errmsg = std::ffi::CString::new(err.to_string()).unwrap();
            purple_connection_error_reason(purple_connection, crate::bridge_structs::PURPLE_CONNECTION_ERROR_OTHER_ERROR, c_errmsg.as_ptr());
        }
    }
}
