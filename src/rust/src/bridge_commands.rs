extern "C" {
    // TODO: automatically generate declaration from presage.h
    fn presage_account_error(
        account: *mut crate::bridge_structs::PurpleAccount,
        reason: crate::bridge_structs::PurpleConnectionError,
        description: *const ::std::os::raw::c_char,
    );
}

unsafe fn send_cmd(
    account: *mut crate::bridge_structs::PurpleAccount,
    rt: *mut tokio::runtime::Runtime,
    tx: *mut tokio::sync::mpsc::Sender<crate::structs::Cmd>,
    cmd: crate::structs::Cmd,
) {
    if let Err(err) = send_cmd_impl(rt, tx, cmd) {
        let errmsg = std::ffi::CString::new(format!("send cmd error: {err}")).unwrap();
        presage_account_error(account, crate::bridge_structs::PURPLE_CONNECTION_ERROR_NETWORK_ERROR, errmsg.as_ptr());
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
    account: *mut crate::bridge_structs::PurpleAccount,
    rt: *mut tokio::runtime::Runtime,
    tx: *mut tokio::sync::mpsc::Sender<crate::structs::Cmd>,
) {
    let cmd = crate::structs::Cmd::Exit {};
    send_cmd(account, rt, tx, cmd);
    // we should be done with this connection instance, drop the box containing the sender
    drop(Box::from_raw(tx));
    // NOTE: the C part should mark their representation of the channel sender as "deleted", too
}

#[no_mangle]
pub unsafe extern "C" fn presage_rust_whoami(
    account: *mut crate::bridge_structs::PurpleAccount,
    rt: *mut tokio::runtime::Runtime,
    tx: *mut tokio::sync::mpsc::Sender<crate::structs::Cmd>,
) {
    let cmd = crate::structs::Cmd::Whoami {};
    send_cmd(account, rt, tx, cmd);
}

// TODO: wire this up completely
#[no_mangle]
pub unsafe extern "C" fn presage_rust_list_groups(
    account: *mut crate::bridge_structs::PurpleAccount,
    rt: *mut tokio::runtime::Runtime,
    tx: *mut tokio::sync::mpsc::Sender<crate::structs::Cmd>,
) {
    let cmd = crate::structs::Cmd::ListGroups {};
    send_cmd(account, rt, tx, cmd);
}

#[no_mangle]
pub unsafe extern "C" fn presage_rust_get_group_members(
    account: *mut crate::bridge_structs::PurpleAccount,
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
            send_cmd(account, rt, tx, cmd);
        }
        Err(err) => {
            let c_errmsg = std::ffi::CString::new(err.to_string()).unwrap();
            presage_account_error(account, crate::bridge_structs::PURPLE_CONNECTION_ERROR_OTHER_ERROR, c_errmsg.as_ptr());
        }
    }
}

#[no_mangle]
pub unsafe extern "C" fn presage_rust_send(
    account: *mut crate::bridge_structs::PurpleAccount,
    rt: *mut tokio::runtime::Runtime,
    tx: *mut tokio::sync::mpsc::Sender<crate::structs::Cmd>,
    c_destination: *const std::os::raw::c_char,
    c_message: *const std::os::raw::c_char,
    xfer: *const crate::bridge_structs::PurpleXfer,
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
            send_cmd(account, rt, tx, cmd);
        }
        Err(err) => {
            let c_errmsg = std::ffi::CString::new(err.to_string()).unwrap();
            presage_account_error(account, crate::bridge_structs::PURPLE_CONNECTION_ERROR_OTHER_ERROR, c_errmsg.as_ptr());
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
    account: *mut crate::bridge_structs::PurpleAccount,
    rt: *mut tokio::runtime::Runtime,
    tx: *mut tokio::sync::mpsc::Sender<crate::structs::Cmd>,
    c_uuid: *const std::os::raw::c_char,
) {
    match presage::libsignal_service::prelude::Uuid::parse_str(std::ffi::CStr::from_ptr(c_uuid).to_str().unwrap()) {
        Ok(uuid) => {
            let cmd = crate::structs::Cmd::GetProfile { uuid };
            send_cmd(account, rt, tx, cmd);
        }
        Err(err) => {
            let c_errmsg = std::ffi::CString::new(err.to_string()).unwrap();
            presage_account_error(account, crate::bridge_structs::PURPLE_CONNECTION_ERROR_OTHER_ERROR, c_errmsg.as_ptr());
        }
    }
}

#[no_mangle]
pub unsafe extern "C" fn presage_rust_get_attachment(
    account: *mut crate::bridge_structs::PurpleAccount,
    rt: *mut tokio::runtime::Runtime,
    tx: *mut tokio::sync::mpsc::Sender<crate::structs::Cmd>,
    attachment_pointer_box: *mut presage::proto::AttachmentPointer,
    xfer: *const crate::bridge_structs::PurpleXfer,
) {
    let attachment_pointer = Box::from_raw(attachment_pointer_box);
    let cmd = crate::structs::Cmd::GetAttachment {
        attachment_pointer: *attachment_pointer,
        xfer: xfer,
    };
    send_cmd(account, rt, tx, cmd);
}

#[no_mangle]
pub unsafe extern "C" fn presage_rust_drop_attachment(attachment_pointer_box: *mut presage::proto::AttachmentPointer) {
    //print!("(xx:xx:xx) presage: presage_rust_drop_attachment({attachment_pointer_box:#?})…\n");
    drop(Box::from_raw(attachment_pointer_box));
}
