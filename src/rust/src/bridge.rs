#[repr(C)]
pub struct Presage {
    pub account: *const std::os::raw::c_void,
    pub tx_ptr: *mut std::os::raw::c_void,
    pub qrcode: *const std::os::raw::c_char,
    pub uuid: *const std::os::raw::c_char,

    // TODO: find out how to use stdint on Windows
    pub timestamp: std::os::raw::c_ulonglong, //stdint::uint64_t,
    pub sent: std::os::raw::c_ulonglong,      //stdint::uint64_t,
    pub who: *const std::os::raw::c_char,
    pub group: *const std::os::raw::c_char,
    pub body: *const std::os::raw::c_char,
}

impl Presage {
    pub fn from_account(account: *const std::os::raw::c_void) -> Self {
        Self {
            account: account,
            tx_ptr: std::ptr::null_mut(),
            qrcode: std::ptr::null(),
            uuid: std::ptr::null(),
            timestamp: 0,
            sent: 0,
            who: std::ptr::null(),
            group: std::ptr::null(),
            body: std::ptr::null(),
        }
    }
}

extern "C" {
    pub fn presage_append_message(input: *const Presage);
}

// https://stackoverflow.com/questions/66196972/how-to-pass-a-reference-pointer-to-a-rust-struct-to-a-c-ffi-interface
#[no_mangle]
pub extern "C" fn presage_rust_init() -> *mut tokio::runtime::Runtime {
    // https://stackoverflow.com/questions/64658556/
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .thread_name("presage Tokio")
        .enable_io()
        .enable_time()
        .build()
        .unwrap();
    let runtime_box = Box::new(runtime);
    Box::into_raw(runtime_box)
}

#[no_mangle]
pub extern "C" fn presage_rust_destroy(runtime: *mut tokio::runtime::Runtime) {
    unsafe {
        drop(Box::from_raw(runtime));
    }
}

#[no_mangle]
pub extern "C" fn presage_rust_free(c_str: *mut std::os::raw::c_char) {
    if c_str == std::ptr::null_mut() {
        return;
    }
    unsafe {
        drop(Box::from_raw(c_str));
    }
}

#[no_mangle]
pub unsafe extern "C" fn presage_rust_main(
    rt: *mut tokio::runtime::Runtime,
    account: *const std::os::raw::c_void,
    c_store_path: *const std::os::raw::c_char,
) {
    let store_path: String = std::ffi::CStr::from_ptr(c_store_path)
        .to_str()
        .unwrap()
        .to_owned();
    let (tx, rx) = tokio::sync::mpsc::channel(32);
    let tx_ptr = Box::into_raw(Box::new(tx));
    let mut message = Presage::from_account(account);
    message.tx_ptr = tx_ptr as *mut std::os::raw::c_void;
    unsafe {
        presage_append_message(&message);
    }
    let runtime = rt.as_ref().unwrap();
    runtime.block_on(async {
        let local = tokio::task::LocalSet::new();
        local
            .run_until(async {
                // from main
                let passphrase: Option<String> = None;
                //println!("rust: opening config database from {store_path}");
                let config_store = presage_store_sled::SledStore::open_with_passphrase(
                    store_path,
                    passphrase,
                    presage_store_sled::MigrationConflictStrategy::Raise,
                );
                match config_store {
                    Err(err) => {
                        println!("rust: config_store Err {err:?}");
                    }
                    Ok(config_store) => {
                        println!("rust: config_store OK");
                        crate::core::mainloop(config_store, rx, account).await;
                    }
                }
            })
            .await;
    });
    println!("rust: main finished.");
}

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
    let device_name: String = std::ffi::CStr::from_ptr(c_device_name)
        .to_str()
        .unwrap()
        .to_owned();
    println!("rust: presage_rust_link invoked successfully! device_name is {device_name}");
    // from args
    let server = presage::prelude::SignalServers::Production;
    //let server = presage::prelude::SignalServers::Staging;
    let cmd = crate::commands::Cmd::LinkDevice {
        device_name: device_name,
        servers: server,
    };
    send_cmd(rt, tx, cmd);
    println!("rust: presage_rust_link ends now");
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
    let cmd= crate::commands::Cmd::Receive {};
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
        uuid: presage::prelude::Uuid::parse_str(std::ffi::CStr::from_ptr(c_uuid).to_str().unwrap())
            .unwrap(),
        message: std::ffi::CStr::from_ptr(c_message)
            .to_str()
            .unwrap()
            .to_owned(),
    };
    send_cmd(rt, tx, cmd);
}
