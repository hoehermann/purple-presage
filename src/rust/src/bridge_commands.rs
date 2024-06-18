/*
 * Feeds a command into the channel c → rust.
 */
unsafe fn send_cmd(
    rt: *mut tokio::runtime::Runtime,
    tx: *mut tokio::sync::mpsc::Sender<crate::commands::Cmd>,
    cmd: crate::commands::Cmd,
) {
    let command_tx = tx.as_ref().unwrap();
    let runtime = rt.as_ref().unwrap();
    match runtime.block_on(command_tx.send(cmd)) {
        Ok(()) => {
            //println!("rust: command_tx.send OK");
        }
        Err(err) => {
            println!("rust: command_tx.send {err}");
        }
    }
}

#[no_mangle]
pub unsafe extern "C" fn presage_rust_link(
    rt: *mut tokio::runtime::Runtime,
    tx: *mut tokio::sync::mpsc::Sender<crate::commands::Cmd>,
    c_device_name: *const std::os::raw::c_char,
) {
    let device_name: String = std::ffi::CStr::from_ptr(c_device_name).to_str().unwrap().to_owned();
    let server = presage::libsignal_service::configuration::SignalServers::Production;
    //let server = presage::libsignal_service::configuration::SignalServers::Staging;
    let cmd = crate::commands::Cmd::LinkDevice {
        device_name: device_name,
        servers: server,
    };
    send_cmd(rt, tx, cmd);
}

#[no_mangle]
pub unsafe extern "C" fn presage_rust_stop(
    rt: *mut tokio::runtime::Runtime,
    tx: *mut tokio::sync::mpsc::Sender<crate::commands::Cmd>,
) {
    let cmd = crate::commands::Cmd::Exit {};
    send_cmd(rt, tx, cmd);
}

#[no_mangle]
pub unsafe extern "C" fn presage_rust_exit(
    rt: *mut tokio::runtime::Runtime,
    tx: *mut tokio::sync::mpsc::Sender<crate::commands::Cmd>,
) {
    let cmd = crate::commands::Cmd::Exit {};
    send_cmd(rt, tx, cmd);
}

#[no_mangle]
pub unsafe extern "C" fn presage_rust_whoami(
    rt: *mut tokio::runtime::Runtime,
    tx: *mut tokio::sync::mpsc::Sender<crate::commands::Cmd>,
) {
    let cmd = crate::commands::Cmd::Whoami {};
    send_cmd(rt, tx, cmd);
}

#[no_mangle]
pub unsafe extern "C" fn presage_rust_receive(
    rt: *mut tokio::runtime::Runtime,
    tx: *mut tokio::sync::mpsc::Sender<crate::commands::Cmd>,
) {
    let cmd = crate::commands::Cmd::Receive {};
    send_cmd(rt, tx, cmd);
}

#[no_mangle]
pub unsafe extern "C" fn presage_rust_send(
    rt: *mut tokio::runtime::Runtime,
    tx: *mut tokio::sync::mpsc::Sender<crate::commands::Cmd>,
    c_uuid: *const std::os::raw::c_char,
    c_message: *const std::os::raw::c_char,
) {
    let cmd = crate::commands::Cmd::Send {
        // TODO: add error handling instead of unwrap()
        uuid: presage::libsignal_service::prelude::Uuid::parse_str(std::ffi::CStr::from_ptr(c_uuid).to_str().unwrap()).unwrap(),
        message: std::ffi::CStr::from_ptr(c_message).to_str().unwrap().to_owned(),
    };
    send_cmd(rt, tx, cmd);
}
