/*
 * Runs a command.
 * 
 * Based on presage-cli's `run`.
 */
async fn run<C: presage::Store + 'static>(
    subcommand: crate::commands::Cmd,
    config_store: C,
    manager: Option<presage::Manager<C, presage::Registered>>,
    account: *const std::os::raw::c_void,
) -> Result<presage::Manager<C, presage::Registered>, presage::Error<<C>::Error>> {
    match subcommand {
        crate::commands::Cmd::LinkDevice {
            servers,
            device_name,
        } => {
            let (provisioning_link_tx, provisioning_link_rx) = futures::channel::oneshot::channel();
            let join_handle = futures::future::join(presage::Manager::link_secondary_device(config_store, servers, device_name.clone(), provisioning_link_tx), async move {
                match provisioning_link_rx.await {
                    Ok(url) => {
                        println!("rust: qr code ok.");
                        println!("rust: now calling presage_append_message…");
                        let mut message = crate::bridge::Presage::from_account(account);
                        message.qrcode = std::ffi::CString::new(url.to_string()).unwrap().into_raw();
                        crate::bridge::append_message(&message);
                    }
                    Err(e) => println!("Error linking device: {e}"),
                }
            })
            .await;

            let mut message = crate::bridge::Presage::from_account(account);
            let qrcode_done = String::from("");
            message.qrcode = std::ffi::CString::new(qrcode_done).unwrap().into_raw();
            crate::bridge::append_message(&message);
            let (manager, _) = join_handle;
            manager
        }

        crate::commands::Cmd::Whoami => {
            let manager = manager.unwrap_or(presage::Manager::load_registered(config_store).await?);
            let whoami = manager.whoami().await?;
            let uuid = whoami.uuid.to_string();
            let mut message = crate::bridge::Presage::from_account(account);
            message.uuid = std::ffi::CString::new(uuid.to_string()).unwrap().into_raw();
            crate::bridge::append_message(&message);
            Ok(manager)
        }

        crate::commands::Cmd::Receive => {
            let manager = manager.unwrap();
            let mut receiving_manager = manager.clone();
            tokio::task::spawn_local(async move { crate::receive_text::receive(&mut receiving_manager, account).await });
            Ok(manager)
        }

        crate::commands::Cmd::Send { uuid, message } => {
            let mut manager = manager.unwrap();
            crate::send_text::send(&message, &uuid, &mut manager).await?;
            Ok(manager)
        }

        crate::commands::Cmd::Exit {} => {
            panic!("Exit command reached inner loop.");
        }
    }
}

/*
 * Retrieves commands from the channel.
 * 
 * Delegates work to `run`, but catches the errors for forwarding to the front-end.
 * 
 * Based on presage-cli's main loop.
 */
pub async fn mainloop(
    config_store: presage_store_sled::SledStore,
    mut rx: tokio::sync::mpsc::Receiver<crate::commands::Cmd>,
    account: *const std::os::raw::c_void,
) {
    let mut manager: Option<presage::Manager<presage_store_sled::SledStore, presage::Registered>> = None;
    while let Some(cmd) = rx.recv().await {
        match cmd {
            crate::commands::Cmd::Exit => {
                break;
            }
            _ => {
                println!("rust: run {:?} begins…", cmd);
                // TODO: find out if config_store.clone() is the correct thing to do here
                match run(cmd, config_store.clone(), manager, account).await {
                    Ok(m) => {
                        manager = Some(m);
                    }
                    Err(presage::Error::NotYetRegisteredError) => {
                        // can happen during whoami
                        manager = None;
                        // tell the front-end we lost authorization
                        let uuid = String::from("");
                        let mut message = crate::bridge::Presage::from_account(account);
                        message.uuid = std::ffi::CString::new(uuid.to_string()).unwrap().into_raw();
                        crate::bridge::append_message(&message);
                    }
                    Err(presage::Error::ServiceError(err)) => {
                        // can happen during whoami or send, possibly others, after main device has revoked the link
                        manager = None;
                        match err {
                            presage::prelude::content::ServiceError::Unauthorized => {
                                // tell the front-end we lost authorization
                                let uuid = String::from("");
                                let mut message = crate::bridge::Presage::from_account(account);
                                message.uuid = std::ffi::CString::new(uuid.to_string()).unwrap().into_raw();
                                crate::bridge::append_message(&message);
                            }
                            _ => {
                                println!("rust: run ServiceError {err:?}");
                            }
                        }
                    }
                    Err(err) => {
                        manager = None;
                        println!("rust: run Err {err:?}");
                    }
                }
                println!("rust: run finished.");
            }
        }
    }
}

/*
 * Opens the store and runs commands forever.
 * 
 * Based on presage-cli's main loop.
 */
pub async fn main(store_path: String, passphrase: Option<String>, rx: tokio::sync::mpsc::Receiver<crate::commands::Cmd>,account: *const std::os::raw::c_void) {
    //println!("rust: opening config database from {store_path}");
    let config_store = presage_store_sled::SledStore::open_with_passphrase(store_path, passphrase, presage_store_sled::MigrationConflictStrategy::Raise);
    match config_store {
        Err(err) => {
            println!("rust: config_store Err {err:?}");
        }
        Ok(config_store) => {
            println!("rust: config_store OK");
            mainloop(config_store, rx, account).await;
        }
    }
}
