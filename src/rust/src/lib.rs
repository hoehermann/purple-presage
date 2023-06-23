use futures::{future, channel::oneshot};
use presage::{
    prelude::{SignalServers,},
    Manager,
};
use presage_store_sled::{SledStore, MigrationConflictStrategy};
use futures::executor::block_on;

use std::ffi::CStr;

#[no_mangle]
pub unsafe extern "C" fn presage_link(c_device_name: *const std::os::raw::c_char) {
    let device_name: String = CStr::from_ptr(c_device_name).to_str().unwrap().to_owned();
    println!("presage_link invoked successfully! device_name is {device_name}");
    
    // from main
    let db_path = "presage";
    let passphrase: Option<String> = None;
    println!("opening config database from {db_path}");
    let config_store = SledStore::open_with_passphrase(
        db_path,
        passphrase,
        MigrationConflictStrategy::Raise,
    );
    
    // from args
    let servers: SignalServers = SignalServers::Staging;
    
    match config_store {
        Ok(config_store) => {
            println!("config_store OK");
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
                            qr2term::print_qr(url.to_string()).expect("failed to render qrcode")
                        }
                        Err(e) => println!("Error linking device: {e}"),
                    }
                },
            );
                
            /*
            let manager = Manager::link_secondary_device(config_store, servers, device_name.clone(), provisioning_link_tx);
            println!("now entering block_on(provisioning_link_rx)…");
            match block_on(provisioning_link_rx) {
                Ok(url) => {
                    qr2term::print_qr(url.to_string()).expect("failed to render qrcode")
                }
                Err(e) => {
                    println!("Error linking device: {e}");
                }
            }*/
            println!("now entering block_on(manager)…");
            // TODO: find out how to use tokio https://stackoverflow.com/questions/66328113/
            match block_on(manager) {
                (Ok(manager), _) => {
                    match block_on(manager.whoami()) {
                        Ok(response) => {
                            let uuid = response.uuid;
                            println!("{uuid:?}");
                        }
                        Err(err) => {
                            println!("{err:?}");
                        }
                    }
                }
                (Err(err), _) => {
                    println!("{err:?}");
                }
            }
        }
        Err(err) => {
            println!("{err:?}");
        }
    }
    
    /*
    async {
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
                        qr2term::print_qr(url.to_string()).expect("failed to render qrcode")
                    }
                    Err(e) => log::error!("Error linking device: {e}"),
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
    }*/
}
