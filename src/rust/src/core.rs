/*
 * Runs a command.
 *
 * Based on presage-cli's `run`.
 */
async fn run<C: presage::store::Store + 'static>(
    subcommand: crate::structs::Cmd,
    config_store: C,
    manager: Option<presage::Manager<C, presage::manager::Registered>>,
    account: *mut crate::bridge_structs::PurpleAccount,
) -> Result<presage::Manager<C, presage::manager::Registered>, presage::Error<<C>::Error>> {
    match subcommand {
        crate::structs::Cmd::LinkDevice {
            servers,
            device_name,
        } => {
            let (provisioning_link_tx, provisioning_link_rx) = futures::channel::oneshot::channel();
            let join_handle = futures::future::join(presage::Manager::link_secondary_device(config_store, servers, device_name.clone(), provisioning_link_tx), async move {
                match provisioning_link_rx.await {
                    Ok(url) => {
                        crate::bridge::purple_debug(account, crate::bridge_structs::PURPLE_DEBUG_INFO, String::from("got URL for QR code\n"));
                        let mut message = crate::bridge_structs::Message::from_account(account);
                        message.qrcode = std::ffi::CString::new(url.to_string()).unwrap().into_raw();
                        crate::bridge::append_message(&message);
                    }
                    Err(err) => {
                        crate::bridge::purple_error(account, crate::bridge_structs::PURPLE_CONNECTION_ERROR_AUTHENTICATION_FAILED, format!("Error linking device: {err:?}"));
                    }
                }
            })
            .await;

            let mut message = crate::bridge_structs::Message::from_account(account);
            let qrcode_done = String::from("");
            message.qrcode = std::ffi::CString::new(qrcode_done).unwrap().into_raw();
            crate::bridge::append_message(&message);
            let (manager, _) = join_handle;
            manager
        }

        crate::structs::Cmd::Whoami => {
            let manager = manager.unwrap_or(presage::Manager::load_registered(config_store).await?);
            let whoami = manager.whoami().await?;
            let uuid = whoami.aci.to_string(); // TODO: check alternatives to aci
            let mut message = crate::bridge_structs::Message::from_account(account);
            message.uuid = std::ffi::CString::new(uuid.to_string()).unwrap().into_raw();
            crate::bridge::append_message(&message);
            Ok(manager)
        }

        crate::structs::Cmd::Receive => {
            let manager = manager.expect("manager must be loaded");
            let mut receiving_manager = manager.clone();
            tokio::task::spawn_local(async move { crate::receive::receive(&mut receiving_manager, account).await });
            Ok(manager)
        }

        crate::structs::Cmd::Send {
            recipient,
            message,
            xfer,
        } => {
            let mut manager = manager.expect("manager must be loaded");
            // prepare a PurplePresage message for providing feed-back (send success or error)
            let mut msg = crate::bridge_structs::Message::from_account(account);
            msg.timestamp = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_millis() as u64;
            msg.xfer = xfer; // in case of attachments, this is the reference to the respective purple Xfer
            match recipient {
                crate::structs::Recipient::Contact(uuid) => {
                    msg.who = std::ffi::CString::new(uuid.to_string()).unwrap().into_raw();
                }
                crate::structs::Recipient::Group(master_key) => {
                    msg.group = std::ffi::CString::new(hex::encode(master_key)).unwrap().into_raw();
                }
            }
            // now do the actual sending and error-handling
            match crate::send::send(&mut manager, recipient, message.clone(), xfer).await {
                Ok(_) => {
                    // NOTE: for Spectrum, send-acknowledgements should be PURPLE_MESSAGE_SEND only (without PURPLE_MESSAGE_REMOTE_SEND)
                    msg.flags = crate::bridge_structs::PurpleMessageFlags::PURPLE_MESSAGE_SEND;
                    if let Some(body) = message {
                        msg.body = std::ffi::CString::new(body).unwrap().into_raw();
                    }
                }
                Err(err) => {
                    // TODO: remove this purple_debug once handling errors is reasonably well tested
                    crate::bridge::purple_debug(
                        account,
                        crate::bridge_structs::PURPLE_DEBUG_ERROR,
                        format!("{err} occurred while sending a message. The error message should appear in the conversation window.\n"),
                    );
                    msg.flags = crate::bridge_structs::PurpleMessageFlags::PURPLE_MESSAGE_ERROR;
                    msg.body = std::ffi::CString::new(err.to_string()).unwrap().into_raw();
                }
            }
            // feed the feed-back back into purple
            crate::bridge::append_message(&msg);
            Ok(manager)
        }

        crate::structs::Cmd::ListGroups => {
            let mut manager = manager.expect("manager must be loaded");
            crate::contacts::get_groups(account, &mut manager).await;
            Ok(manager)
        }

        crate::structs::Cmd::GetGroupMembers { master_key_bytes } => crate::contacts::get_group_members(account, manager, master_key_bytes).await,

        crate::structs::Cmd::Exit {} => {
            crate::bridge::purple_error(account, crate::bridge_structs::PURPLE_CONNECTION_ERROR_OTHER_ERROR, String::from("Exit command reached inner loop."));
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
    mut rx: tokio::sync::mpsc::Receiver<crate::structs::Cmd>,
    account: *mut crate::bridge_structs::PurpleAccount,
) {
    let mut manager: Option<presage::Manager<presage_store_sled::SledStore, presage::manager::Registered>> = None;
    while let Some(cmd) = rx.recv().await {
        match cmd {
            crate::structs::Cmd::Exit => {
                break;
            }
            _ => {
                //purple_debug(account, crate::bridge_structs::PURPLE_DEBUG_INFO, format!("run {:?} begins…\n", cmd));
                // TODO: find out if config_store.clone() is the correct thing to do here
                match run(cmd.clone(), config_store.clone(), manager, account).await {
                    Ok(m) => {
                        manager = Some(m);
                    }
                    Err(presage::Error::NotYetRegisteredError) => {
                        // can happen during whoami
                        manager = None;
                        // tell the front-end we lost authorization
                        let uuid = String::from("");
                        let mut message = crate::bridge_structs::Message::from_account(account);
                        message.uuid = std::ffi::CString::new(uuid.to_string()).unwrap().into_raw();
                        crate::bridge::append_message(&message);
                    }
                    Err(presage::Error::ServiceError(err)) => {
                        // can happen during whoami or send, possibly others, after main device has revoked the link
                        manager = None;
                        match err {
                            presage::libsignal_service::push_service::ServiceError::Unauthorized => {
                                // tell the front-end we lost authorization
                                let uuid = String::from("");
                                let mut message = crate::bridge_structs::Message::from_account(account);
                                message.uuid = std::ffi::CString::new(uuid.to_string()).unwrap().into_raw();
                                crate::bridge::append_message(&message);
                            }
                            _ => {
                                crate::bridge::purple_error(account, crate::bridge_structs::PURPLE_CONNECTION_ERROR_OTHER_ERROR, format!("run unhandled ServiceError {err:?}"));
                            }
                        }
                    }
                    Err(err) => {
                        manager = None;
                        crate::bridge::purple_error(account, crate::bridge_structs::PURPLE_CONNECTION_ERROR_OTHER_ERROR, format!("run Err {err:?}"));
                    }
                }
                //purple_debug(account, crate::bridge_structs::PURPLE_DEBUG_INFO, format!("run {:?} finished.\n", cmd));
            }
        }
    }
}

/*
 * Opens the store and runs commands forever.
 *
 * Based on presage-cli's main loop.
 */
pub async fn main(
    store_path: String,
    passphrase: Option<String>,
    rx: tokio::sync::mpsc::Receiver<crate::structs::Cmd>,
    account: *mut crate::bridge_structs::PurpleAccount,
) {
    crate::bridge::purple_debug(account, crate::bridge_structs::PURPLE_DEBUG_INFO, format!("opening config database from {store_path}\n"));
    let config_store = presage_store_sled::SledStore::open_with_passphrase(
        store_path,
        passphrase,
        presage_store_sled::MigrationConflictStrategy::Raise,
        presage::model::identity::OnNewIdentity::Trust,
    );
    match config_store.await {
        Err(err) => {
            crate::bridge::purple_error(account, crate::bridge_structs::PURPLE_CONNECTION_ERROR_OTHER_ERROR, format!("config_store Err {err:#?}"));
        }
        Ok(config_store) => {
            crate::bridge::purple_debug(account, crate::bridge_structs::PURPLE_DEBUG_INFO, String::from("config_store OK\n"));
            mainloop(config_store, rx, account).await;
        }
    }
}
