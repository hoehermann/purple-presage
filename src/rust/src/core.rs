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

pub fn purple_error(account: *const std::os::raw::c_void, level:i32, msg: String) {
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
pub fn purple_debug(account: *const std::os::raw::c_void, level:i32, msg: String) {
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

        crate::structs::Cmd::Whoami => {
            let manager = manager.unwrap_or(presage::Manager::load_registered(config_store).await?);
            let whoami = manager.whoami().await?;
            let uuid = whoami.uuid.to_string();
            let mut message = crate::bridge::Presage::from_account(account);
            message.uuid = std::ffi::CString::new(uuid.to_string()).unwrap().into_raw();
            crate::bridge::append_message(&message);
            Ok(manager)
        }

        crate::structs::Cmd::Receive => {
            let manager = manager.unwrap();
            let mut receiving_manager = manager.clone();
            tokio::task::spawn_local(async move { crate::receive_text::receive(&mut receiving_manager, account).await });
            Ok(manager)
        }

        crate::structs::Cmd::Send { recipient, message } => {
            let mut manager = manager.unwrap();
            crate::send_text::send(&mut manager, recipient, &message).await?;
            Ok(manager)
        }

        crate::structs::Cmd::Exit {} => {
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
                purple_debug(account, 2, format!("run {:?} begins…\n", cmd));
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
                            presage::libsignal_service::push_service::ServiceError::Unauthorized => {
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
                purple_debug(account, 2, String::from("run finished.\n"));
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
    //println!("rust: opening config database from {store_path}");
    let config_store =
        presage_store_sled::SledStore::open_with_passphrase(store_path, passphrase, presage_store_sled::MigrationConflictStrategy::Raise, presage_store_sled::OnNewIdentity::Trust);
    match config_store {
        Err(err) => {
            purple_error(account, 16, format!("config_store Err {err:?}"));
        }
        Ok(config_store) => {
            purple_debug(account, 2, String::from("config_store OK\n"));
            mainloop(config_store, rx, account).await;
        }
    }
}
