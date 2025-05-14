/*
 * Runs a command.
 *
 * Based on presage-cli's `run`.
 */
async fn run<C: presage::store::Store + 'static>(
    subcommand: crate::structs::Cmd,
    mut manager: presage::Manager<C, presage::manager::Registered>,
    account: *mut crate::bridge_structs::PurpleAccount,
) -> Result<bool, presage::Error<<C>::Error>> {
    match subcommand {
        crate::structs::Cmd::Whoami => {
            let whoami = manager.whoami().await?;
            let uuid = whoami.aci.to_string(); // TODO: check alternatives to aci
            let mut message = crate::bridge_structs::Message::from_account(account);
            message.uuid = std::ffi::CString::new(uuid.to_string()).unwrap().into_raw();
            crate::bridge::append_message(&message);
            Ok(true)
        }

        crate::structs::Cmd::Send {
            recipient,
            message,
            xfer,
        } => {
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
                        format!("Error „{err}“ occurred while sending a message. The error message should appear in the conversation window.\n"),
                    );
                    msg.flags = crate::bridge_structs::PurpleMessageFlags::PURPLE_MESSAGE_ERROR;
                    let errmsg = err.to_string(); // TODO: prefix error message with "Error: "
                    msg.body = std::ffi::CString::new(errmsg).unwrap().into_raw();
                }
            }
            // feed the feed-back back into purple
            crate::bridge::append_message(&msg);
            Ok(true)
        }

        crate::structs::Cmd::ListGroups => {
            crate::contacts::forward_groups(account, &mut manager).await;
            Ok(true)
        }

        crate::structs::Cmd::GetGroupMembers { master_key_bytes } => {
            crate::contacts::get_group_members(account, manager, master_key_bytes).await?;
            Ok(true)
        }

        crate::structs::Cmd::GetProfile { uuid } => {
            match manager.store().contact_by_id(&uuid).await {
                Err(err) => crate::bridge::purple_debug(
                    account,
                    crate::bridge_structs::PURPLE_DEBUG_ERROR,
                    format!("Error while looking up contact information for {uuid}: {err}\n"),
                ),
                Ok(contact) => match contact {
                    None => crate::bridge::purple_debug(account, crate::bridge_structs::PURPLE_DEBUG_WARNING, format!("No contact information available for {uuid}.\n")),
                    Some(contact) => {
                        let mut message = crate::bridge_structs::Message::from_account(account);
                        message.who = std::ffi::CString::new(contact.uuid.to_string()).unwrap().into_raw();
                        message.name = if contact.name != "" { std::ffi::CString::new(contact.name).unwrap().into_raw() } else { std::ptr::null_mut() };
                        message.phone_number = contact.phone_number.map_or(std::ptr::null_mut(), |pn| std::ffi::CString::new(pn.to_string()).unwrap().into_raw());
                        crate::bridge::append_message(&message);
                    }
                },
            }
            Ok(true)
        }

        crate::structs::Cmd::Exit {} => Ok(false),
    }
}

/*
 * Retrieves both: Commands from the command channel and messages from the receiver.
 *
 * Delegates work to `run`, but catches the errors for forwarding to the front-end.
 *
 * Based on presage-cli's main loop and flare's manager thread.
 */
pub async fn mainloop<C: presage::store::Store + 'static>(
    mut manager: presage::Manager<C, presage::manager::Registered>,
    mut command_receiver: tokio::sync::mpsc::Receiver<crate::structs::Cmd>,
    account: *mut crate::bridge_structs::PurpleAccount,
) {
    crate::contacts::forward_contacts(account, &mut manager).await;
    crate::bridge::purple_debug(account, crate::bridge_structs::PURPLE_DEBUG_INFO, format!("mainloop begins…\n"));
    let messages = manager.receive_messages().await.expect("receive_messages failed");
    crate::bridge::purple_debug(account, crate::bridge_structs::PURPLE_DEBUG_INFO, format!("messages ok\n"));

    futures::pin_mut!(messages);
    let mut keep_running = true;
    while keep_running {
        tokio::select! {
            maybe_cmd = command_receiver.recv() => {
                crate::bridge::purple_debug(account, crate::bridge_structs::PURPLE_DEBUG_INFO, format!("cmd: {maybe_cmd:?}\n"));
                match maybe_cmd {
                    Some(cmd) =>  {
                        match run(cmd, manager.clone(), account).await {
                            Ok(keep_running_commands) => {
                                crate::bridge::purple_debug(account, crate::bridge_structs::PURPLE_DEBUG_INFO, format!("cmd ok.\n"));
                                keep_running = keep_running_commands;
                            },
                            Err(err) => {
                                crate::bridge::purple_error(account, crate::bridge_structs::PURPLE_CONNECTION_ERROR_OTHER_ERROR, format!("run Err {err:?}"));
                            },
                        }
                    },
                    None => {
                        crate::bridge::purple_error(account, crate::bridge_structs::PURPLE_CONNECTION_ERROR_NETWORK_ERROR, format!("Command channel disrupted."));
                        keep_running = false;
                    }
                }
            },
            maybe_received = futures::StreamExt::next(&mut messages) => {
                crate::bridge::purple_debug(account, crate::bridge_structs::PURPLE_DEBUG_INFO, format!("received: {maybe_received:?}\n"));
                match maybe_received {
                    Some(received) => crate::receive::handle_received(&mut manager, account, received).await,
                    None => {
                        // this happens when the main device unlinks this device
                        // this also happens spuriously, perhaps due to network issues
                        // re-connecting is a good idea in either case, so we forward a network error to purple
                        crate::bridge::purple_error(account, crate::bridge_structs::PURPLE_CONNECTION_ERROR_NETWORK_ERROR, format!("Receiver was disconnected."));
                        keep_running = false;
                    }
                }
            }
        }
    }
}

pub async fn login(
    config_store: presage_store_sled::SledStore,
    account: *mut crate::bridge_structs::PurpleAccount,
) -> Option<presage::Manager<presage_store_sled::SledStore, presage::manager::Registered>> {
    crate::bridge::purple_debug(account, crate::bridge_structs::PURPLE_DEBUG_INFO, format!("login begins…\n"));
    match presage::Manager::load_registered(config_store.clone()).await {
        Ok(manager) => {
            crate::bridge::purple_debug(account, crate::bridge_structs::PURPLE_DEBUG_INFO, format!("manager ok\n"));
            return Some(manager);
        }
        Err(presage::Error::NotYetRegisteredError) => {
            // happens on pristine set-ups
            // can happen during whoami
            return link(config_store, account).await;
        }
        Err(presage::Error::ServiceError(err)) => {
            // can happen during load_registered after main device has revoked the link
            // NOTE: possibly also happens during execution of commands like whoami or send, possibly others
            match err {
                presage::libsignal_service::push_service::ServiceError::Unauthorized => {
                    return link(config_store, account).await;
                }
                _ => {
                    crate::bridge::purple_error(account, crate::bridge_structs::PURPLE_CONNECTION_ERROR_OTHER_ERROR, format!("login ServiceError {err:?}"));
                }
            }
        }
        Err(err) => {
            crate::bridge::purple_error(account, crate::bridge_structs::PURPLE_CONNECTION_ERROR_OTHER_ERROR, format!("login error {err:?}"));
        }
    }
    crate::bridge::purple_debug(account, crate::bridge_structs::PURPLE_DEBUG_INFO, format!("login failed.\n"));
    None
}

async fn link(
    config_store: presage_store_sled::SledStore,
    account: *mut crate::bridge_structs::PurpleAccount,
) -> Option<presage::Manager<presage_store_sled::SledStore, presage::manager::Registered>> {
    let device_name = "purple-presage".to_string(); // TODO: use hostname or make user-configurable
    let server = presage::libsignal_service::configuration::SignalServers::Production;
    let (provisioning_link_tx, provisioning_link_rx) = futures::channel::oneshot::channel();
    let join_handle = futures::future::join(presage::Manager::link_secondary_device(config_store, server, device_name, provisioning_link_tx), async move {
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
    let (manager, _) = join_handle;

    match manager {
        Ok(mut manager) => {
            let whoami = manager.whoami().await; // this seems to be necessary for the manager to finish the linking process
            match whoami {
                Ok(whoami) => {
                    let uuid = whoami.aci.to_string(); // TODO: check if there are alternatives to aci
                    let mut message = crate::bridge_structs::Message::from_account(account);
                    message.uuid = std::ffi::CString::new(uuid.to_string()).unwrap().into_raw();
                    crate::bridge::append_message(&message);

                    // request contacts now after linking once. requesting again on a subsequent log-in sometimes blocks forever.
                    if let Err(err) = manager.request_contacts().await {
                        crate::bridge::purple_debug(account, crate::bridge_structs::PURPLE_DEBUG_INFO, format!("Error while requesting contacts: {err:?}\n"));
                    }

                    return Some(manager);
                }
                Err(err) => {
                    crate::bridge::purple_error(
                        account,
                        crate::bridge_structs::PURPLE_CONNECTION_ERROR_AUTHENTICATION_FAILED,
                        format!("Error checking identity: {err:?}"),
                    );
                }
            }
        }
        Err(err) => {
            crate::bridge::purple_error(
                account,
                crate::bridge_structs::PURPLE_CONNECTION_ERROR_AUTHENTICATION_FAILED,
                format!("Error after linking device: {err:?}"),
            );
        }
    }
    return None;
}

/*
 * Opens the store, does the log-in, then runs forever.
 *
 * Based on presage-cli's main loop.
 */
pub async fn main(
    store_path: String,
    passphrase: Option<String>,
    command_receiver: tokio::sync::mpsc::Receiver<crate::structs::Cmd>,
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
            crate::bridge::purple_error(account, crate::bridge_structs::PURPLE_CONNECTION_ERROR_OTHER_ERROR, format!("config store error {err:#?}"));
        }
        Ok(config_store) => {
            crate::bridge::purple_debug(account, crate::bridge_structs::PURPLE_DEBUG_INFO, String::from("config store OK\n"));

            /*
            At this point, we tell the front-end that the account is "connected". We need to do this since the blist's aliasing functions do not work on disconnected accounts.
            Also, Spectrum2 allegedly needs the account to be connected else prosody refuses to forward the message with the code string necessary for linking.
            On Spectrum2, the accound must not be marked as connected before the C → rust channel has been set-up since Spectrum2 will start requesting the room list immediately.
            However, the connection is not fully usable, yet. The presage docs at https://github.com/whisperfish/presage/blob/3f55d5f/presage/src/manager/registered.rs#L574 state:
            „As a client, it is heavily recommended to process incoming messages and wait for the Received::QueueEmpty messages before giving the ability for users to send messages.“
            */
            let mut message = crate::bridge_structs::Message::from_account(account);
            message.connected = 1;
            crate::bridge::append_message(&message);

            if let Some(manager) = login(config_store, account).await {
                mainloop(manager, command_receiver, account).await;
            }
        }
    }
}
