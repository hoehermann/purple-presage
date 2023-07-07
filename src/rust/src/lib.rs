use futures::{future, channel::oneshot};
use presage::{
    prelude::{SignalServers,},
    Manager,
};
use presage_store_sled::{SledStore, MigrationConflictStrategy};


#[repr(C)]
pub struct Presage {
    pub account: *const std::os::raw::c_void,
    //pub tx_ptr: *mut tokio::sync::mpsc::Sender<Cmd>,
    pub tx_ptr: *mut std::os::raw::c_void,
    pub qrcode: *const std::os::raw::c_char,
}

extern "C" {
    fn presage_append_message(input: *const Presage);
}

// https://stackoverflow.com/questions/66196972/how-to-pass-a-reference-pointer-to-a-rust-struct-to-a-c-ffi-interface
#[no_mangle]
pub extern fn presage_rust_init() -> *mut tokio::runtime::Runtime {
    // https://stackoverflow.com/questions/64658556/
    let runtime = tokio::runtime::Builder::new_multi_thread().thread_name("presage Tokio").enable_io().enable_time().build().unwrap();
    let runtime_box = Box::new(runtime);
    Box::into_raw(runtime_box)
}

#[no_mangle]
pub extern fn presage_rust_destroy(runtime: *mut tokio::runtime::Runtime) {
    unsafe { drop(Box::from_raw(runtime)); }
}

use presage::Store;

// from main
pub enum Cmd {
    LinkDevice {
        servers: SignalServers,
        device_name: String,
    },
    Whoami,
}

async fn run<C: Store + 'static>(subcommand: Cmd, config_store: C, account: *const std::os::raw::c_void) {
    match subcommand {
        Cmd::LinkDevice {
            servers,
            device_name,
        } => {
            let (provisioning_link_tx, provisioning_link_rx) = oneshot::channel();
            let manager = future::join(
                Manager::link_secondary_device(
                    config_store,
                    servers,
                    device_name.clone(),
                    provisioning_link_tx,
                ),
                async move {
                    match provisioning_link_rx.await {
                        Ok(url) => {
                            println!("rust: qr code ok.");
                            println!("rust: now calling presage_append_message…");
                            let message = Presage{
                                account: account, 
                                tx_ptr: std::ptr::null_mut(),
                                qrcode: std::ffi::CString::new(url.to_string()).unwrap().into_raw()
                            };
                            unsafe { presage_append_message(&message); }
                        }
                        Err(e) => println!("Error linking device: {e}"),
                    }
                },
            )
            .await;

            match manager {
                (Ok(manager), _) => {
                    let uuid = manager.whoami().await.unwrap().uuid;
                    println!("{uuid:?}");
                }
                (Err(err), _) => {
                    println!("{err:?}");
                }
            }
        }
        Cmd::Whoami => {
            let _manager = Manager::load_registered(config_store).await;
            //println!("{:?}", &manager.whoami().await);
        }
    }
}

#[no_mangle]
pub unsafe extern "C" fn presage_rust_main(rt: *mut tokio::runtime::Runtime, account: *const std::os::raw::c_void) {
    println!("rust: presage_rust_main for account {account:p}");
    let (tx, mut rx) = tokio::sync::mpsc::channel(32);
    let tx_ptr = Box::into_raw(Box::new(tx));
    println!("rust: tx_ptr is now {tx_ptr:p}");
    let message = Presage{
        account: account, 
        tx_ptr: tx_ptr as *mut std::os::raw::c_void,
        qrcode: std::ptr::null()
    };
    println!("rust: now calling presage_append_message…");
    unsafe { presage_append_message(&message); }
    println!("rust: presage_append_message has returned");
    let runtime = rt.as_ref().unwrap();
    runtime.block_on(async {
        while let Some(cmd) = rx.recv().await {
            // from main
            let db_path = "presage";
            let passphrase: Option<String> = None;
            println!("rust: opening config database from {db_path}");
            let config_store = SledStore::open_with_passphrase(
                db_path,
                passphrase,
                MigrationConflictStrategy::Raise,
            );
            match config_store {
                Ok(config_store) => {
                    println!("rust: config_store OK");
                    run(cmd, config_store, account).await
                }
                Err(err) => {
                    println!("rust: {err:?}");
                }
            }
        }
    });
}

// let mut manager = Manager::load_registered(config_store).await?;

#[no_mangle]
pub unsafe extern "C" fn presage_rust_link(rt: *mut tokio::runtime::Runtime, tx : *mut tokio::sync::mpsc::Sender<Cmd>, c_device_name: *const std::os::raw::c_char) {
    let device_name: String = std::ffi::CStr::from_ptr(c_device_name).to_str().unwrap().to_owned();
    println!("rust: presage_rust_link invoked successfully! device_name is {device_name}");
    
    // from args
    //let server: SignalServers = SignalServers::Production;
    let server: SignalServers = SignalServers::Staging;
    let cmd: Cmd = Cmd::LinkDevice {device_name: device_name, servers: server};

    let command_tx = tx.as_ref().unwrap();
    let runtime = rt.as_ref().unwrap();
    match runtime.block_on(command_tx.send(cmd)) {
        Ok(()) => {
            println!("rust: command_tx.send OK");
        }
        Err(err) => {
            println!("rust: command_tx.send {err}");
        }
    }
    
    println!("rust: presage_rust_link ends now");
}
