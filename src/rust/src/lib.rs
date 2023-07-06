use futures::{future, channel::oneshot};
use presage::{
    prelude::{SignalServers,},
    Manager,
};
use presage_store_sled::{SledStore, MigrationConflictStrategy};

extern "C" {
    fn presage_process_message_bridge(input: *const std::os::raw::c_char);
}

/*
#[derive(Default)]
#[repr(C)]
pub struct PresageConnection{
    runtime
}


impl PresageConnection{
    pub fn init(&mut self) {
        //
    }
}*/

// https://stackoverflow.com/questions/66196972/how-to-pass-a-reference-pointer-to-a-rust-struct-to-a-c-ffi-interface
#[no_mangle]
pub extern fn presage_rust_init() -> *mut tokio::runtime::Runtime {
    /*
    let mut pc = PresageConnection::default();
    pc.init();
    */
    // https://stackoverflow.com/questions/64658556/how-do-i-use-a-custom-tokio-runtime-within-tokio-postgres-and-without-the-tokio
    let runtime = tokio::runtime::Builder::new_multi_thread().thread_name("presage Tokio").enable_io().enable_time().build().unwrap();
    let runtime_box = Box::new(runtime);
    Box::into_raw(runtime_box)
}

#[no_mangle]
pub extern fn presage_rust_destroy(runtime: *mut tokio::runtime::Runtime) {
    unsafe { drop(Box::from_raw(runtime)); }
}

#[no_mangle]
pub unsafe extern "C" fn presage_rust_link(rt: *mut tokio::runtime::Runtime, c_device_name: *const std::os::raw::c_char) {
    let runtime = rt.as_ref().unwrap();
    
    let device_name: String = std::ffi::CStr::from_ptr(c_device_name).to_str().unwrap().to_owned();
    println!("presage presage_link invoked successfully! device_name is {device_name}");
    
    // from main
    let db_path = "presage";
    let passphrase: Option<String> = None;
    println!("presage opening config database from {db_path}");
    let config_store = SledStore::open_with_passphrase(
        db_path,
        passphrase,
        MigrationConflictStrategy::Raise,
    );
    
    // from args
    let servers: SignalServers = SignalServers::Production;//Staging;
    
    match config_store {
        Ok(config_store) => {
            println!("presage config_store OK");
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
                            println!("presage now calling presage_process_message_bridge…");
                            presage_process_message_bridge(c_qrcodedata.as_ptr());
                        }
                        Err(e) => println!("presage Error linking device: {e}"),
                    }
                },
            );
                
            println!("presage now entering block_on(manager)…");
            match runtime.block_on(manager) {
                (Ok(manager), _) => {
                    println!("presage now entering block_on(manager.whoami())…");
                    match runtime.block_on(manager.whoami()) {
                        Ok(response) => {
                            let uuid = response.uuid;
                            println!("presage {uuid:?}");
                        }
                        Err(err) => {
                            println!("presage {err:?}");
                        }
                    }
                }
                (Err(err), _) => {
                    println!("presage {err:?}");
                }
            }
        }
        Err(err) => {
            println!("presage {err:?}");
        }
    }
    
    println!("presage_link ends now");
}
