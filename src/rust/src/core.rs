// TODO: purple_error and purple_debug should probably be in the bridge module
/*
Look at these error levels from Purple:

typedef enum
{
    PURPLE_CONNECTION_ERROR_NETWORK_ERROR = 0,
    PURPLE_CONNECTION_ERROR_INVALID_USERNAME = 1,
    PURPLE_CONNECTION_ERROR_AUTHENTICATION_FAILED = 2,
    PURPLE_CONNECTION_ERROR_AUTHENTICATION_IMPOSSIBLE = 3,
    PURPLE_CONNECTION_ERROR_NO_SSL_SUPPORT = 4,
    PURPLE_CONNECTION_ERROR_ENCRYPTION_ERROR = 5,
    PURPLE_CONNECTION_ERROR_NAME_IN_USE = 6,
    PURPLE_CONNECTION_ERROR_INVALID_SETTINGS = 7,
    PURPLE_CONNECTION_ERROR_CERT_NOT_PROVIDED = 8,
    PURPLE_CONNECTION_ERROR_CERT_UNTRUSTED = 9,
    PURPLE_CONNECTION_ERROR_CERT_EXPIRED = 10,
    PURPLE_CONNECTION_ERROR_CERT_NOT_ACTIVATED = 11,
    PURPLE_CONNECTION_ERROR_CERT_HOSTNAME_MISMATCH = 12,
    PURPLE_CONNECTION_ERROR_CERT_FINGERPRINT_MISMATCH = 13,
    PURPLE_CONNECTION_ERROR_CERT_SELF_SIGNED = 14,
    PURPLE_CONNECTION_ERROR_CERT_OTHER_ERROR = 15,
    PURPLE_CONNECTION_ERROR_OTHER_ERROR = 16
} PurpleConnectionError;

TODO: Automatically convert from libpurple/connection.h.
*/

pub fn purple_error(
    account: *const std::os::raw::c_void,
    level: i32,
    msg: String,
) {
    let mut message = crate::bridge::Presage::from_account(account);
    message.error = level;
    message.body = std::ffi::CString::new(msg).unwrap().into_raw();
    crate::bridge::append_message(&message);
}

/*
Look at these debug levels from Purple:

typedef enum
{
    PURPLE_DEBUG_ALL = 0,  /**< All debug levels.              */
    PURPLE_DEBUG_MISC,     /**< General chatter.               */
    PURPLE_DEBUG_INFO,     /**< General operation Information. */
    PURPLE_DEBUG_WARNING,  /**< Warnings.                      */
    PURPLE_DEBUG_ERROR,    /**< Errors.                        */
    PURPLE_DEBUG_FATAL     /**< Fatal errors.                  */
} PurpleDebugLevel;

TODO: Automatically convert from libpurple/debug.h.
*/
pub fn purple_debug(
    account: *const std::os::raw::c_void,
    level: i32,
    msg: String,
) {
    let mut message = crate::bridge::Presage::from_account(account);
    message.debug = level;
    message.body = std::ffi::CString::new(msg).unwrap().into_raw();
    crate::bridge::append_message(&message);
}

/*
 * Runs a command.
 *
 * Based on presage-cli's `run`.
 */
async fn run<C: presage::store::Store + 'static>(
    subcommand: crate::structs::Cmd,
    config_store: C,
    manager: Option<presage::Manager<C, presage::manager::Registered>>,
    account: *const std::os::raw::c_void,
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
                        purple_debug(account, 2, String::from("got URL for QR code\n"));
                        let mut message = crate::bridge::Presage::from_account(account);
                        message.qrcode = std::ffi::CString::new(url.to_string()).unwrap().into_raw();
                        crate::bridge::append_message(&message);
                    }
                    Err(err) => {
                        crate::core::purple_error(account, 2 /* PURPLE_CONNECTION_ERROR_AUTHENTICATION_FAILED */, format!("Error linking device: {err:?}"));
                    }
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

        crate::structs::Cmd::Whoami => {
            let manager = manager.unwrap_or(presage::Manager::load_registered(config_store).await?);
            let whoami = manager.whoami().await?;
            let uuid = whoami.aci.to_string(); // TODO: check alternatives to aci
            let mut message = crate::bridge::Presage::from_account(account);
            message.uuid = std::ffi::CString::new(uuid.to_string()).unwrap().into_raw();
            crate::bridge::append_message(&message);
            Ok(manager)
        }

        crate::structs::Cmd::InitialSync => {
            let mut manager = manager.expect("manager must be loaded");
            let messages = manager.receive_messages(presage::manager::ReceivingMode::InitialSync).await;
            match messages {
                Ok(_) => {
                    // TODO: handle the messages. there might be something useful in there
                    crate::core::purple_debug(account, 2, format!("InitialSync completed.\n"));

                    // also, fetch contacts and groups now
                    manager = crate::contacts::get_contacts(account, Some(manager)).await?;
                    manager = crate::contacts::get_groups(account, Some(manager)).await?;

                    // now that the initial sync has completed,
                    // the connection can be regarded as "connected" and ready to send messages
                    let mut message = crate::bridge::Presage::from_account(account);
                    message.connected = 1;
                    crate::bridge::append_message(&message);
                }
                Err(err) => {
                    crate::core::purple_error(account, 16, format!("InitialSync error {err:?}"));
                }
            }
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
            let mut msg = crate::bridge::Presage::from_account(account);
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
                    msg.flags = 0x0001; // PURPLE_MESSAGE_SEND
                    if let Some(body) = message {
                        msg.body = std::ffi::CString::new(body).unwrap().into_raw();
                    }
                }
                Err(err) => {
                    // TODO: remove this purple_debug once handling errors is reasonably well tested
                    purple_debug(
                        account,
                        4,
                        format!("{err} occurred while sending a message. The error message should appear in the conversation window.\n"),
                    );
                    msg.flags = 0x0200; // PURPLE_MESSAGE_ERROR
                    msg.body = std::ffi::CString::new(err.to_string()).unwrap().into_raw();
                }
            }
            // feed the feed-back back into purple
            crate::bridge::append_message(&msg);
            Ok(manager)
        }

        crate::structs::Cmd::ListGroups => crate::contacts::get_groups(account, manager).await,

        crate::structs::Cmd::GetGroupMembers { master_key_bytes } => crate::contacts::get_group_members(account, manager, master_key_bytes).await,

        crate::structs::Cmd::Exit {} => {
            purple_error(account, 16, String::from("Exit command reached inner loop."));
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
    account: *const std::os::raw::c_void,
) {
    let mut manager: Option<presage::Manager<presage_store_sled::SledStore, presage::manager::Registered>> = None;
    while let Some(cmd) = rx.recv().await {
        match cmd {
            crate::structs::Cmd::Exit => {
                break;
            }
            _ => {
                //purple_debug(account, 2, format!("run {:?} begins…\n", cmd));
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
                        let mut message = crate::bridge::Presage::from_account(account);
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
                                let mut message = crate::bridge::Presage::from_account(account);
                                message.uuid = std::ffi::CString::new(uuid.to_string()).unwrap().into_raw();
                                crate::bridge::append_message(&message);
                            }
                            _ => {
                                purple_error(account, 16, format!("run unhandled ServiceError {err:?}"));
                            }
                        }
                    }
                    Err(err) => {
                        manager = None;
                        purple_error(account, 16, format!("run Err {err:?}"));
                    }
                }
                //purple_debug(account, 2, format!("run {:?} finished.\n", cmd));
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
    account: *const std::os::raw::c_void,
) {
    purple_debug(account, 2, format!("opening config database from {store_path}\n"));
    let config_store =
        presage_store_sled::SledStore::open_with_passphrase(store_path, passphrase, presage_store_sled::MigrationConflictStrategy::Raise, presage::model::identity::OnNewIdentity::Trust);
    match config_store.await {
        Err(err) => {
            purple_error(account, 16, format!("config_store Err {err:?}"));
        }
        Ok(config_store) => {
            purple_debug(account, 2, String::from("config_store OK\n"));
            mainloop(config_store, rx, account).await;
        }
    }
}
