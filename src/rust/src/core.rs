use std::error::Error;

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
            crate::bridge::append_message(crate::bridge::Message {
                account: account,
                uuid: Some(uuid.to_string()),
                ..Default::default()
            });
            Ok(true)
        }
        crate::structs::Cmd::Send {
            recipient,
            message,
            xfer,
        } => {
            // prepare a PurplePresage message for providing feed-back (send success or error)
            let mut msg = crate::bridge::Message {
                account: account,
                timestamp: Some(std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_millis() as u64),
                xfer: xfer, // in case of attachments, this is the reference to the respective purple Xfer
                ..Default::default()
            };
            match recipient {
                crate::structs::Recipient::Contact(uuid) => {
                    msg.who = Some(uuid.to_string());
                }
                crate::structs::Recipient::Group(master_key) => {
                    msg.group = Some(hex::encode(master_key));
                }
            }
            // now do the actual sending and error-handling
            match crate::send::send(&mut manager, recipient, message.clone(), xfer).await {
                Ok(_) => {
                    // NOTE: for Spectrum, send-acknowledgements should be PURPLE_MESSAGE_SEND only (without PURPLE_MESSAGE_REMOTE_SEND)
                    msg.flags = crate::bridge_structs::PurpleMessageFlags::PURPLE_MESSAGE_SEND;
                    if let Some(body) = message {
                        msg.body = Some(body);
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
                    msg.body = Some(format!("Error: {err}"));
                }
            }
            // feed the feed-back back into purple
            crate::bridge::append_message(msg);
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
                        let name = if contact.name.is_empty() { None } else { Some(contact.name) };
                        let phone_number = contact.phone_number.map(|pn| pn.to_string());
                        crate::bridge::append_message(crate::bridge::Message {
                            account: account,
                            who: Some(contact.uuid.to_string()),
                            name: name,
                            phone_number: phone_number,
                            ..Default::default()
                        });
                    }
                },
            }
            Ok(true)
        }
        crate::structs::Cmd::GetAttachment {
            attachment_pointer,
            xfer,
        } => {
            crate::attachment::get_attachment(account, manager, attachment_pointer, xfer).await;
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
    crate::bridge::purple_debug(account, crate::bridge_structs::PURPLE_DEBUG_INFO, format!("mainloop begins…\n"));
    let messages = manager.receive_messages().await.expect("receive_messages failed");
    crate::bridge::purple_debug(account, crate::bridge_structs::PURPLE_DEBUG_INFO, format!("messages ok\n"));

    futures::pin_mut!(messages);
    let mut keep_running = true;
    while keep_running {
        tokio::select! {
            maybe_cmd = command_receiver.recv() => {
                match maybe_cmd {
                    Some(cmd) =>  {
                        match run(cmd, manager.clone(), account).await {
                            Ok(keep_running_commands) => {
                                keep_running = keep_running_commands;
                            },
                            Err(err) => {
                                crate::bridge::purple_error(account, crate::bridge_structs::PURPLE_CONNECTION_ERROR_OTHER_ERROR, format!("run Err {err:?}"));
                            },
                        }
                    },
                    None => {
                        // this should never happen
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
    config_store: presage_store_sqlite::SqliteStore,
    account: *mut crate::bridge_structs::PurpleAccount,
) -> Option<presage::Manager<presage_store_sqlite::SqliteStore, presage::manager::Registered>> {
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
                // Handle specific HTTP timeout error – by ChatGPT
                presage::libsignal_service::push_service::ServiceError::Http(ref http_err) => {
                    // Check if the error is a timeout
                    if let Some(source) = std::error::Error::source(&http_err) {
                        // Try to downcast to a timeout error type
                        if source.downcast_ref::<std::io::Error>().map_or(false, |io_err| io_err.kind() == std::io::ErrorKind::TimedOut) {
                            crate::bridge::purple_error(
                                account,
                                crate::bridge_structs::PURPLE_CONNECTION_ERROR_NETWORK_ERROR,
                                format!("Network timeout while logging in: {http_err:?}"),
                            );
                            return None;
                        }
                        if source.downcast_ref::<reqwest::Error>().map_or(false, |reqwest_err| reqwest_err.source().map_or(false, |source| source.downcast_ref::<hyper_util::client::legacy::Error>().map_or(false, |client_error| client_error.is_connect()))) {
                            crate::bridge::purple_error(
                                account,
                                crate::bridge_structs::PURPLE_CONNECTION_ERROR_NETWORK_ERROR,
                                format!("Client connect error while logging in: {http_err:?}"),
                            );
                            return None;
                        }
                    }
                    // Fallback for other HTTP errors
                    crate::bridge::purple_error(account, crate::bridge_structs::PURPLE_CONNECTION_ERROR_OTHER_ERROR, format!("login ServiceError {http_err:?}"));
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
    config_store: presage_store_sqlite::SqliteStore,
    account: *mut crate::bridge_structs::PurpleAccount,
) -> Option<presage::Manager<presage_store_sqlite::SqliteStore, presage::manager::Registered>> {
    let device_name = "purple-presage".to_string(); // TODO: use hostname or make user-configurable
    let server = presage::libsignal_service::configuration::SignalServers::Production;
    let (provisioning_link_tx, provisioning_link_rx) = futures::channel::oneshot::channel();
    let join_handle = futures::future::join(presage::Manager::link_secondary_device(config_store, server, device_name, provisioning_link_tx), async move {
        match provisioning_link_rx.await {
            Ok(url) => {
                crate::bridge::purple_debug(account, crate::bridge_structs::PURPLE_DEBUG_INFO, String::from("got URL for QR code\n"));
                crate::bridge::append_message(crate::bridge::Message {
                    account: account,
                    qrcode: Some(url.to_string()),
                    ..Default::default()
                });
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
                    crate::bridge::append_message(crate::bridge::Message {
                        account: account,
                        uuid: Some(uuid),
                        ..Default::default()
                    });

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
    let config_store = presage_store_sqlite::SqliteStore::open_with_passphrase(&store_path, passphrase.as_deref(), presage::model::identity::OnNewIdentity::Trust);
    match config_store.await {
        Err(err) => {
            crate::bridge::purple_error(account, crate::bridge_structs::PURPLE_CONNECTION_ERROR_OTHER_ERROR, format!("config store error {err:#?}"));
        }
        Ok(config_store) => {
            crate::bridge::purple_debug(account, crate::bridge_structs::PURPLE_DEBUG_INFO, String::from("config store OK\n"));
            if let Some(mut manager) = login(config_store, account).await {
                // Login has succeeded, forward (cached) contacts for bitlbee. It tends to forget them after re-connects.
                crate::contacts::forward_contacts(account, &mut manager).await;
                mainloop(manager, command_receiver, account).await;
            }
        }
    }
}
