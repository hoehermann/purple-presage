/**
 * A local representation of a message that is probably going to be forwarded to the front-end.
 */
#[derive(Clone)]
pub struct Message {
    thread: Option<presage::store::Thread>,
    account: *mut crate::bridge_structs::PurpleAccount,
    timestamp: Option<u64>,
    flags: crate::bridge_structs::PurpleMessageFlags,
    who: Option<String>,
    name: Option<String>,
    group: Option<String>,
    body: Option<String>,
}
impl Message {
    fn into_bridge(
        self,
        body: Option<String>,
    ) -> crate::bridge_structs::Message {
        // TODO: Do this in append_message, but not with into_raw, but with as_ptr, then g_strdup in presage_append_message.
        // Then the presage_rust_free_* functions are no longer needed.
        let to_cstr_or_null = |s: Option<String>| -> *mut ::std::os::raw::c_char { s.map_or(std::ptr::null_mut(), |s| std::ffi::CString::new(s).unwrap().into_raw()) };
        let body = body.or(self.body);
        crate::bridge_structs::Message {
            account: self.account,
            tx_ptr: std::ptr::null_mut(),
            qrcode: std::ptr::null_mut(),
            uuid: std::ptr::null_mut(),
            debug: -1,
            error: -1,
            connected: -1,
            padding: -1,
            timestamp: self.timestamp.unwrap_or_default() as u64,
            flags: self.flags,
            who: to_cstr_or_null(self.who),
            name: to_cstr_or_null(self.name),
            phone_number: std::ptr::null_mut(),
            group: to_cstr_or_null(self.group),
            body: to_cstr_or_null(body),
            blob: std::ptr::null_mut(),
            size: 0,
            groups: std::ptr::null_mut(),
            roomlist: std::ptr::null_mut(),
            xfer: std::ptr::null_mut(),
        }
    }
}

/**
Looks up the title of a group identified by its group master key.

Adapted from presage-cli.
*/
async fn format_group<S: presage::store::Store>(
    key: [u8; 32],
    manager: &presage::Manager<S, presage::manager::Registered>,
) -> String {
    manager.store().group(key).await.ok().flatten().map(|g| g.title).unwrap_or_else(|| "<missing group>".to_string())
}

async fn lookup_message_body_by_timestamp<S: presage::store::Store>(
    manager: &presage::Manager<S, presage::manager::Registered>,
    thread: &presage::store::Thread,
    timestamp: u64,
) -> Option<String> {
    match manager.store().message(thread, timestamp).await {
        Err(_) => None,
        Ok(None) => None,
        Ok(Some(message)) => {
            if let presage::libsignal_service::content::ContentBody::DataMessage(presage::libsignal_service::content::DataMessage { body, .. })
            | presage::libsignal_service::content::ContentBody::SynchronizeMessage(presage::libsignal_service::content::SyncMessage {
                sent:
                    Some(presage::proto::sync_message::Sent {
                        message: Some(presage::libsignal_service::content::DataMessage { body, .. }),
                        ..
                    }),
                ..
            }) = message.body
            {
                body
            } else {
                None
            }
        }
    }
}

/**
Turns a DataMessage into a string for presentation via libpurple.

Adapted from presage-cli.
*/
async fn format_data_message<C: presage::store::Store>(
    manager: &mut presage::Manager<C, presage::manager::Registered>,
    account: *mut crate::bridge_structs::PurpleAccount,
    thread: &presage::store::Thread,
    data_message: &presage::libsignal_service::content::DataMessage,
) -> Option<String> {
    match data_message {
        // Quote
        presage::libsignal_service::content::DataMessage {
            quote: Some(presage::proto::data_message::Quote {
                text: Some(quoted_text),
                ..
            }),
            body: Some(body),
            ..
        } => {
            let firstline = quoted_text.split("\n").next().unwrap_or("<message body missing>");
            // TODO: add ellipsis if quoted_text contains more than one line
            Some(format!("> {firstline}\n\n{body}"))
        }
        // Reaction
        presage::libsignal_service::content::DataMessage {
            reaction:
                Some(presage::proto::data_message::Reaction {
                    target_sent_timestamp: Some(timestamp),
                    emoji: Some(emoji),
                    ..
                }),
            ..
        } => {
            match lookup_message_body_by_timestamp(manager, thread, *timestamp).await {
                None => {
                    let sent_at =
                        chrono::prelude::DateTime::<chrono::Local>::from(std::time::UNIX_EPOCH + std::time::Duration::from_millis(*timestamp)).format("%Y-%m-%d %H:%M:%S");
                    Some(format!("Reacted with {emoji} to message from {sent_at}."))
                }
                Some(body) => {
                    let firstline = body.split("\n").next().unwrap_or("<message body missing>");
                    // TODO: add ellipsis if body contains more than one line
                    Some(format!("Reacted with {emoji} to message „{firstline}“."))
                }
            }
        }
        // Plain text message
        // TODO: resolve mentions
        presage::libsignal_service::content::DataMessage {
            body: Some(body), ..
        } => Some(body.to_string()),
        // Default (catch all other cases)
        c => {
            crate::bridge::purple_debug(account, crate::bridge_structs::PURPLE_DEBUG_INFO, format!("DataMessage without body {c:?}\n"));
            // NOTE: This happens when receiving a file, but not providing a text
            // TODO: suppress this debug message if data message contained an attachment
            // NOTE: flags: Some(4) with a timestamp (and a profile_key?) may indicate "message sent"
            // Some("message has been sent".to_string())
            None
        }
    }
}

async fn process_attachments<C: presage::store::Store>(
    manager: &mut presage::Manager<C, presage::manager::Registered>,
    message: Message,
    attachments: &Vec<presage::proto::AttachmentPointer>,
) {
    let account = message.account;
    for attachment_pointer in attachments {
        let Ok(attachment_data) = manager.get_attachment(attachment_pointer).await else {
            let mut message = message.clone().into_bridge(Some("Failed to fetch attachment.".to_string()));
            message.flags = crate::bridge_structs::PurpleMessageFlags::PURPLE_MESSAGE_ERROR;
            crate::bridge::append_message(&message);
            continue;
        };

        match attachment_pointer.content_type.as_deref() {
            None => {
                crate::bridge::purple_debug(account, crate::bridge_structs::PURPLE_DEBUG_ERROR, format!("Received attachment without content type.\n"));
            }
            Some("text/x-signal-plain") => {
                // TODO: this should be routed through the function that usually handles the text messages
                // strip trailing null bytes, thanks to https://stackoverflow.com/questions/49406517/how-to-remove-trailing-null-characters-from-string#comment139692696_49406848
                let body = std::ffi::CStr::from_bytes_until_nul(&attachment_data).unwrap().to_str().unwrap().to_owned();
                crate::bridge::append_message(&message.clone().into_bridge(Some(body)));
            }
            Some(mimetype) => {
                let extension = match mimetype {
                    "image/jpeg" => "jpg",
                    "image/png" => "png",
                    "video/mp4" => "mp4",
                    mimetype => {
                        let extensions = mime_guess::get_mime_extensions_str(mimetype);
                        extensions.and_then(|e| e.first()).unwrap_or(&"bin")
                    }
                };

                let filename = match attachment_pointer.attachment_identifier.clone().unwrap() {
                    presage::proto::attachment_pointer::AttachmentIdentifier::CdnId(id) => id.to_string(),
                    presage::proto::attachment_pointer::AttachmentIdentifier::CdnKey(key) => key,
                };
                let boxed_slice = attachment_data.into_boxed_slice();
                let mut message = message.clone().into_bridge(None);
                message.size = boxed_slice.len() as usize;
                message.name = std::ffi::CString::new(format!("{filename}.{extension}")).unwrap().into_raw();
                message.blob = Box::into_raw(boxed_slice) as *mut std::os::raw::c_void;
                crate::bridge::append_message(&message);
            }
        }
    }
}

async fn process_data_message<C: presage::store::Store>(
    manager: &mut presage::Manager<C, presage::manager::Registered>,
    message: Message,
    data_message: &presage::proto::DataMessage,
) -> Option<String> {
    process_attachments(manager, message.clone(), &data_message.attachments).await;
    format_data_message(manager, message.account, &message.thread.unwrap(), data_message).await
}

async fn process_sent_message<C: presage::store::Store>(
    manager: &mut presage::Manager<C, presage::manager::Registered>,
    message: Message,
    sent: &presage::proto::sync_message::Sent,
) {
    let mut message = message;
    message.flags = crate::bridge_structs::PurpleMessageFlags::PURPLE_MESSAGE_SEND | crate::bridge_structs::PurpleMessageFlags::PURPLE_MESSAGE_REMOTE_SEND;
    if let Some(body) = match sent {
        presage::proto::sync_message::Sent {
            message: Some(data_message),
            ..
        } => process_data_message(manager, message.clone(), data_message).await,
        presage::proto::sync_message::Sent {
            edit_message: Some(presage::proto::EditMessage {
                data_message: Some(data_message),
                ..
            }),
            ..
        } => process_data_message(manager, message.clone(), &data_message).await,
        c => {
            crate::bridge::purple_debug(message.account, crate::bridge_structs::PURPLE_DEBUG_WARNING, format!("Unsupported message {c:?}\n"));
            None
        }
    } {
        crate::bridge::append_message(&message.into_bridge(Some(body)));
    }
}

async fn process_sync_message<C: presage::store::Store>(
    manager: &mut presage::Manager<C, presage::manager::Registered>,
    message: Message,
    sync_message: &presage::proto::SyncMessage,
) {
    // TODO: explicitly ignore SynchronizeMessage(SyncMessage { sent: None, contacts: None, request: None, read: [], blocked: None, verified: None, configuration: None, padding: Some([…]), …, delete_for_me: Some(DeleteForMe { message_deletes: [MessageDeletes { conversation: Some(ConversationIdentifier { identifier: Some(ThreadServiceId("REDACTED")) }), messages: [AddressableMessage { sent_timestamp: Some(1674147919685), author: Some(AuthorServiceId("REDACTED")) }] }], conversation_deletes: [], local_only_conversation_deletes: [], attachment_deletes: [] }) })
    if let Some(sent) = &sync_message.sent {
        process_sent_message(manager, message, sent).await;
    }
}

async fn process_received_message<C: presage::store::Store>(
    manager: &mut presage::Manager<C, presage::manager::Registered>,
    message: Message,
    received: &presage::libsignal_service::content::ContentBody,
) {
    if let Some(body) = match received {
        presage::libsignal_service::content::ContentBody::NullMessage(_) => Some("Null message (for example deleted)".to_string()),
        presage::libsignal_service::content::ContentBody::DataMessage(data_message) => process_data_message(manager, message.clone(), data_message).await,
        presage::libsignal_service::content::ContentBody::SynchronizeMessage(_) => {
            panic!("SynchronizeMessage ended up in process_received_message!")
        }
        presage::libsignal_service::content::ContentBody::CallMessage(_) => Some("is calling!".to_string()),
        // TODO: forward these properly
        presage::libsignal_service::content::ContentBody::TypingMessage(_) => None, // TODO Some(Msg::Received(&thread, "is typing...".into())), // too annyoing for now. also does not differentiate between "started typing" and "stopped typing"
        presage::libsignal_service::content::ContentBody::ReceiptMessage(_) => None, // TODO Some(Msg::Received(&thread, "received a message.".into())), // works, but too annyoing for now
        presage::libsignal_service::content::ContentBody::EditMessage(_) => None,    // TODO
        c => {
            crate::bridge::purple_debug(message.account, crate::bridge_structs::PURPLE_DEBUG_WARNING, format!("Unsupported message {c:?}\n"));
            None
        }
    } {
        crate::bridge::append_message(&message.into_bridge(Some(body)));
    }
}

/*
 * Prepares a received message (text and attachments) for further processing.
 *
 * Based on presage-cli's `process_incoming_message`.
 */
async fn process_incoming_message<C: presage::store::Store>(
    manager: &mut presage::Manager<C, presage::manager::Registered>,
    content: &presage::libsignal_service::content::Content,
    account: *mut crate::bridge_structs::PurpleAccount,
) {
    let Ok(thread) = presage::store::Thread::try_from(content) else {
        crate::bridge::purple_error(account, crate::bridge_structs::PURPLE_CONNECTION_ERROR_OTHER_ERROR, String::from("failed to find conversation"));
        return;
    };
    let mut message = Message {
        account: account,
        timestamp: None,
        who: None,
        group: None,
        flags: crate::bridge_structs::PurpleMessageFlags(0),
        body: None,
        name: None,
        thread: None,
    };
    message.thread = Some(thread.clone());
    message.timestamp = Some(content.metadata.timestamp);
    match thread {
        presage::store::Thread::Contact(uuid) => {
            message.who = Some(uuid.to_string());
        }
        presage::store::Thread::Group(key) => {
            // TODO: check if this who works for sync messages
            message.who = Some(content.metadata.sender.raw_uuid().to_string());
            message.group = Some(hex::encode(key));
            message.name = Some(format_group(key, manager).await);
        }
    }

    match &content.body {
        presage::libsignal_service::content::ContentBody::SynchronizeMessage(sync_message) => process_sync_message(manager, message, sync_message).await,
        _ => {
            message.flags = crate::bridge_structs::PurpleMessageFlags::PURPLE_MESSAGE_RECV;
            process_received_message(manager, message, &content.body).await
        }
    }
}

/*
 * Receives messages from Signal servers.
 *
 * Blocks forever.
 *
 * Based on presage-cli's `receive`.
 */
pub async fn receive<S: presage::store::Store>(
    manager: &mut presage::Manager<S, presage::manager::Registered>,
    account: *mut crate::bridge_structs::PurpleAccount,
) {
    crate::bridge::purple_debug(account, crate::bridge_structs::PURPLE_DEBUG_INFO, String::from("receive begins…\n"));
    match manager.receive_messages().await {
        Err(err) => {
            crate::bridge::purple_error(
                account,
                crate::bridge_structs::PURPLE_CONNECTION_ERROR_OTHER_ERROR,
                format!("failed to receive messaged due to {err:?}"),
            );
        }
        Ok(messages) => {
            // to the front-end, we now claim to be connected since libpurple's blist and aliasing functions do not work on offline accounts at all
            // technically, this is wrong since the back-end has not completed key synchronization and cannot yet send messages, but here we are
            let mut message = crate::bridge_structs::Message::from_account(account);
            message.connected = 1;
            crate::bridge::append_message(&message);

            futures::pin_mut!(messages);
            // NOTE: This blocks until there is a message to be handled. Blocking forever seems to be by design.
            while let Some(content) = futures::StreamExt::next(&mut messages).await {
                match content {
                    presage::model::messages::Received::QueueEmpty => {
                        // this happens once after all old messages have been received and processed
                        crate::bridge::purple_debug(account, crate::bridge_structs::PURPLE_DEBUG_INFO, format!("synchronization completed.\n"));

                        // now that the initial sync has completed,
                        // the account can be regarded as "connected" since it is ready to send messages
                        let mut message = crate::bridge_structs::Message::from_account(account);
                        message.connected = 1;
                        crate::bridge::append_message(&message);

                        // forward contacts and groups to front-end now
                        // these need to happen *after* setting the account to "connected" due to a purple_account_is_connected in purple_blist_find_chat
                        crate::contacts::get_contacts(account, manager).await;
                        crate::contacts::get_groups(account, manager).await;
                    }
                    presage::model::messages::Received::Contacts => {
                        crate::bridge::purple_debug(account, crate::bridge_structs::PURPLE_DEBUG_INFO, format!("got contacts synchronization.\n"));
                        // NOTE: I never saw this happening.
                        // TODO: Check if this happens during linking.
                    }
                    presage::model::messages::Received::Content(content) => process_incoming_message(manager, &content, account).await,
                }
            }
        }
    }
    crate::bridge::purple_error(
        account,
        crate::bridge_structs::PURPLE_CONNECTION_ERROR_NETWORK_ERROR,
        String::from("Receiver has finished. Disconnected?"),
    );
}
