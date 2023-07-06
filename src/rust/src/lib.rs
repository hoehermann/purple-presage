use futures::{future, channel::oneshot};
use presage::{
    prelude::{SignalServers,},
    Manager,
};
use presage_store_sled::{SledStore, MigrationConflictStrategy};

use std::ffi::CStr;

/*
const QUIET_ZONE_WIDTH: usize = 2;
fn qr2string<D: AsRef<[u8]>>(data: D) -> Result<(), qr2term::QrError> {
    // Generate QR code pixel matrix
    let mut matrix = qr2term::qr::Qr::from(data)?.to_matrix();
    matrix.surround(QUIET_ZONE_WIDTH, qr2term::render::QrLight);
    let cursor = std::io::Cursor::new(Vec::<u8>::new());
    // Render QR code to Cursor
    //qr2term::Renderer::default().render(matrix, &mut cursor).expect("failed to render QR code into string");
    Ok(())
}*/

extern "C" {
    fn presage_process_message_bridge(input: *const std::os::raw::c_char);
}

#[no_mangle]
pub unsafe extern "C" fn presage_link(c_device_name: *const std::os::raw::c_char) {
    // https://stackoverflow.com/questions/64658556/how-do-i-use-a-custom-tokio-runtime-within-tokio-postgres-and-without-the-tokio
    // TODO: do this once in init.
    let rt = tokio::runtime::Builder::new_multi_thread().thread_name("presage Tokio").enable_io().enable_time().build().unwrap();
    
    let device_name: String = CStr::from_ptr(c_device_name).to_str().unwrap().to_owned();
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
                            //qr2term::print_qr().expect("presage failed to render qrcode")
                        }
                        Err(e) => println!("presage Error linking device: {e}"),
                    }
                },
            );
                
            println!("presage now entering block_on(manager)…");
            match rt.block_on(manager) {
                (Ok(manager), _) => {
                    println!("presage now entering block_on(manager.whoami())…");
                    match rt.block_on(manager.whoami()) {
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
