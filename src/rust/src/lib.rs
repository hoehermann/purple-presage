use futures::{future, channel::oneshot};
use presage::{
    prelude::{SignalServers,},
    Manager,
};
use presage_store_sled::{SledStore, MigrationConflictStrategy};

extern "C" {
    fn presage_append_message(input: *const std::os::raw::c_char);
}

/*
#[derive(Default)]
#[repr(C)]
pub struct PresageConnection{
    config_store: Store
}


impl PresageConnection{
    pub fn init(&mut self) {
    }
}*/
    /*
    let mut pc = PresageConnection::default();
    pc.init();
    */

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

async fn run<C: Store + 'static>(subcommand: Cmd, config_store: C) {
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
                            println!("presage qr code ok.");
                            let c_qrcodedata = std::ffi::CString::new(url.to_string()).unwrap();
                            println!("presage now calling presage_append_message…");
                            unsafe { presage_append_message(c_qrcodedata.as_ptr()); }
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

#[repr(C)]
pub struct Presage {
    pub account: *mut std::os::raw::c_void,
    pub tx_ptr: *mut tokio::sync::mpsc::Sender<Cmd>,
}

#[no_mangle]
pub unsafe extern "C" fn presage_rust_main(rt: *mut tokio::runtime::Runtime, presage_ptr: *mut Presage) {
    let (tx, mut rx) = tokio::sync::mpsc::channel(32);
    let tx_box = Box::new(tx);
    let tx_ptr = Box::into_raw(tx_box);
    (*presage_ptr).tx_ptr = tx_ptr;
    println!("rust: tx_ptr is now {tx_ptr:p}");
    let msg = std::ffi::CString::new("eyyoooo").unwrap();
    println!("presage now calling presage_append_message…");
    unsafe { presage_append_message(msg.as_ptr()); }
    let runtime = rt.as_ref().unwrap();
    runtime.block_on(async {
        while let Some(cmd) = rx.recv().await {
            // from main
            let db_path = "presage";
            let passphrase: Option<String> = None;
            println!("presage opening config database from {db_path}");
            let config_store = SledStore::open_with_passphrase(
                db_path,
                passphrase,
                MigrationConflictStrategy::Raise,
            );
            match config_store {
                Ok(config_store) => {
                    println!("presage config_store OK");
                    run(cmd, config_store).await
                }
                Err(err) => {
                    println!("presage {err:?}");
                }
            }
        }
    });
}

// let mut manager = Manager::load_registered(config_store).await?;

#[no_mangle]
pub unsafe extern "C" fn presage_rust_link(rt: *mut tokio::runtime::Runtime, tx : *mut tokio::sync::mpsc::Sender<Cmd>, c_device_name: *const std::os::raw::c_char) {
    let device_name: String = std::ffi::CStr::from_ptr(c_device_name).to_str().unwrap().to_owned();
    println!("presage presage_rust_link invoked successfully! device_name is {device_name}");
    
    // from args
    let server: SignalServers = SignalServers::Production;//Staging;
    let cmd: Cmd = Cmd::LinkDevice {device_name: device_name, servers: server};

    let command_tx = tx.as_ref().unwrap();
    let runtime = rt.as_ref().unwrap();
    match runtime.block_on(command_tx.send(cmd)) {
        Ok(()) => {
            println!("presage command_tx.send OK");
        }
        Err(err) => {
            println!("presage command_tx.send {err}");
        }
    }
    
    println!("presage presage_rust_link ends now");
}
