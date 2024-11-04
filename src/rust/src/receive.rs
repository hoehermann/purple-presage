use futures::StreamExt; // for Stream.next()

/**
Looks up the title of a group identified by its group master key.

Adapted from presage-cli.
*/
async fn format_group<S: presage::store::Store>(key: [u8; 32], manager: &presage::Manager<S, presage::manager::Registered>) -> String {
    manager
    .store()
    .group(key)
    .await
    .ok()
    .flatten()
    .map(|g| g.title)
    .unwrap_or_else(|| "<missing group>".to_string())
}

/**
Looks up the display name for a contact identified by their uuid.

Adapted from presage-cli.
*/
async fn format_contact<S: presage::store::Store>(uuid: &presage::libsignal_service::prelude::Uuid, manager: &presage::Manager<S, presage::manager::Registered>) -> String {
    manager
    .store()
    .contact_by_id(uuid)
    .await
    .ok()
    .flatten()
    .filter(|c| !c.name.is_empty())
    .map(|c| c.name)
    .unwrap_or_else(|| uuid.to_string())
}

/**
Turns a DataMessage into a string for presentation via libpurple.

Adapted from presage-cli.
*/
async fn format_data_message<S: presage::store::Store>(
    thread: &presage::store::Thread,
    data_message: &presage::libsignal_service::content::DataMessage,
    manager: &presage::Manager<S, presage::manager::Registered>,
    account: *const std::os::raw::c_void,
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
            let Ok(Some(message)) = manager.store().message(thread, *timestamp).await else {
                // Original message could not be found. As a best effort, give some reference by displaying the timestamp.
                let sent_at =
                    chrono::prelude::DateTime::<chrono::Local>::from(std::time::UNIX_EPOCH + std::time::Duration::from_millis(*timestamp)).format("%Y-%m-%d %H:%M:%S");
                return Some(format!("Reacted with {emoji} to message from {sent_at}."));
            };

            let (presage::libsignal_service::content::ContentBody::DataMessage(presage::libsignal_service::content::DataMessage {
                body: Some(body), ..
            })
            | presage::libsignal_service::content::ContentBody::SynchronizeMessage(presage::libsignal_service::content::SyncMessage {
                sent:
                    Some(presage::proto::sync_message::Sent {
                        message: Some(presage::libsignal_service::content::DataMessage {
                            body: Some(body), ..
                        }),
                        ..
                    }),
                ..
            })) = message.body
            else {
                // Sometimes, synced messages are not resolved here and reactions to them end up in this arm.
                let sent_at =
                    chrono::prelude::DateTime::<chrono::Local>::from(std::time::UNIX_EPOCH + std::time::Duration::from_millis(*timestamp)).format("%Y-%m-%d %H:%M:%S");
                return Some(format!("Reacted with {emoji} to message from {sent_at}."));
            };
            let firstline = body.split("\n").next().unwrap_or("<message body missing>");
            // TODO: add ellipsis if body contains more than one line
            Some(format!("Reacted with {emoji} to message „{firstline}“."))
        }
        // Plain text message
        // TODO: resolve mentions
        presage::libsignal_service::content::DataMessage {
            body: Some(body), ..
        } => Some(body.to_string()),
        // Default (catch all other cases)
        c => {
            crate::core::purple_debug(account, 2, format!("DataMessage without body {c:?}\n"));
            // NOTE: This happens when receiving a file, but not providing a text
            // TODO: suppress this debug message if data message contained an attachment
            // NOTE: flags: Some(4) with a timestamp (and a profile_key?) may indicate "message sent"
            // Some("message has been sent".to_string())
            None
        }
    }
}

/**
Prepares a received message's text for the front-end.

Adapted from presage-cli.
*/
async fn print_message<C: presage::store::Store>(
    manager: &presage::Manager<C, presage::manager::Registered>,
    content: &presage::libsignal_service::content::Content,
    account: *const std::os::raw::c_void,
) {
    crate::core::purple_debug(account, 2, String::from("print_message called…\n"));
    let Ok(thread) = presage::store::Thread::try_from(content) else {
        crate::core::purple_error(account, 16, String::from("failed to derive thread from content"));
        return;
    };

    enum Msg<'a> {
        Received(&'a presage::store::Thread, String),
        Sent(&'a presage::store::Thread, String),
    }

    if let Some(msg) = match &content.body {
        presage::libsignal_service::content::ContentBody::NullMessage(_) => Some(Msg::Received(&thread, "Null message (for example deleted)".to_string())),
        presage::libsignal_service::content::ContentBody::DataMessage(data_message) => format_data_message(&thread, data_message, manager, account).await.map(|body| Msg::Received(&thread, body)),
        presage::libsignal_service::content::ContentBody::SynchronizeMessage(presage::libsignal_service::content::SyncMessage {
            sent: Some(presage::proto::sync_message::Sent {
                message: Some(data_message),
                ..
            }),
            ..
        }) => format_data_message(&thread, data_message, manager, account).await.map(|body| Msg::Sent(&thread, body)),
        presage::libsignal_service::content::ContentBody::CallMessage(_) => Some(Msg::Received(&thread, "is calling!".into())),
        // TODO: forward these properly
        presage::libsignal_service::content::ContentBody::TypingMessage(_) => None, //Some(Msg::Received(&thread, "is typing...".into())), // too annyoing for now. also does not differentiate between "started typing" and "stopped typing"
        presage::libsignal_service::content::ContentBody::ReceiptMessage(_) => None, //Some(Msg::Received(&thread, "received a message.".into())), // works, but too annyoing for now
        // TODO: explicitly ignore SynchronizeMessage(SyncMessage { sent: None, contacts: None, request: None, read: [], blocked: None, verified: None, configuration: None, padding: Some([…]), …, delete_for_me: Some(DeleteForMe { message_deletes: [MessageDeletes { conversation: Some(ConversationIdentifier { identifier: Some(ThreadServiceId("REDACTED")) }), messages: [AddressableMessage { sent_timestamp: Some(1674147919685), author: Some(AuthorServiceId("REDACTED")) }] }], conversation_deletes: [], local_only_conversation_deletes: [], attachment_deletes: [] }) })
        c => {
            crate::core::purple_debug(account, 2, format!("Unsupported message {c:?}\n"));
            None
        }
    } {
        let mut message = crate::bridge::Presage::from_account(account);
        message.timestamp = content.metadata.timestamp;
        match msg {
            // NOTE: for Spectrum, synced messages sent from other own device must set flags PURPLE_MESSAGE_SEND and PURPLE_MESSAGE_REMOTE_SEND
            Msg::Received(presage::store::Thread::Contact(sender), body) => {
                message.flags = 0x0002; // PURPLE_MESSAGE_RECV
                message.who = std::ffi::CString::new(sender.to_string()).unwrap().into_raw();
                message.name = std::ffi::CString::new(format_contact(sender, manager).await).unwrap().into_raw();
                message.body = std::ffi::CString::new(body).unwrap().into_raw();
            }
            Msg::Sent(presage::store::Thread::Contact(recipient), body) => {
                message.flags = 0x0001 | 0x10000; // PURPLE_MESSAGE_SEND | PURPLE_MESSAGE_REMOTE_SEND
                message.who = std::ffi::CString::new(recipient.to_string()).unwrap().into_raw();
                message.body = std::ffi::CString::new(body).unwrap().into_raw();
            }
            Msg::Received(presage::store::Thread::Group(key), body) => {
                message.flags = 0x0002; // PURPLE_MESSAGE_RECV
                message.who = std::ffi::CString::new(content.metadata.sender.uuid.to_string()).unwrap().into_raw();
                message.name = std::ffi::CString::new(format_contact(&content.metadata.sender.uuid, manager).await).unwrap().into_raw();
                message.group = std::ffi::CString::new(hex::encode(key)).unwrap().into_raw();
                message.title = std::ffi::CString::new(format_group(*key, manager).await).unwrap().into_raw();
                message.body = std::ffi::CString::new(body).unwrap().into_raw();
            }
            Msg::Sent(presage::store::Thread::Group(key), body) => {
                message.flags = 0x0001 | 0x10000; // PURPLE_MESSAGE_SEND | PURPLE_MESSAGE_REMOTE_SEND
                message.group = std::ffi::CString::new(hex::encode(key)).unwrap().into_raw();
                message.title = std::ffi::CString::new(format_group(*key, manager).await).unwrap().into_raw();
                message.body = std::ffi::CString::new(body).unwrap().into_raw();
            }
        };
        crate::bridge::append_message(&message);
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
    account: *const std::os::raw::c_void,
) {
    print_message(manager, content, account).await;

    if let presage::libsignal_service::content::ContentBody::DataMessage(presage::libsignal_service::content::DataMessage { attachments, .. })
    | presage::libsignal_service::content::ContentBody::SynchronizeMessage(presage::libsignal_service::content::SyncMessage {
        sent: Some(presage::proto::sync_message::Sent {
            message: Some(presage::libsignal_service::content::DataMessage { attachments, .. }),
            ..
        }),
        ..
    }) = &content.body
    {
        for attachment_pointer in attachments {
            let mut message = crate::bridge::Presage::from_account(account);
            message.timestamp = content.metadata.timestamp;
            // TODO: `who` and `group` should be filled with the Receiver (group or contact) information
            // so they end up in the correct conversation
            // relevant for sync messages in particular
            message.who = std::ffi::CString::new(content.metadata.sender.uuid.to_string()).unwrap().into_raw();

            let Ok(attachment_data) = manager.get_attachment(attachment_pointer).await else {
                message.flags = 0x0200; // PURPLE_MESSAGE_ERROR
                message.body = std::ffi::CString::new(String::from("Failed to fetch attachment.")).unwrap().into_raw();
                crate::bridge::append_message(&message);
                continue;
            };

            let mimetype = attachment_pointer.content_type.as_deref().unwrap_or("application/octet-stream");
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
            message.name = std::ffi::CString::new(format!("{filename}.{extension}")).unwrap().into_raw();
            let boxed_slice = attachment_data.into_boxed_slice();
            message.size = boxed_slice.len() as u64; // TODO: blobsize should be a C type compatible with usize
            message.blob = Box::into_raw(boxed_slice) as *const std::os::raw::c_uchar;
            crate::bridge::append_message(&message);
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
pub async fn receive<C: presage::store::Store>(
    manager: &mut presage::Manager<C, presage::manager::Registered>,
    account: *const std::os::raw::c_void,
) {
    //crate::core::purple_debug(account, 2, String::from("receive on separate thread begins…\n"));
    let messages = manager.receive_messages(presage::manager::ReceivingMode::Forever).await;
    match messages {
        Ok(messages) => {
            //crate::core::purple_debug(account, 2, String::from("receive got messages\n"));
            futures::pin_mut!(messages);
            while let Some(content) = messages.next().await {
                // NOTE: This blocks until there is a message to be handled. Blocking forever seems to be by design.
                //crate::core::purple_debug(account, 2, String::from("receive got a message's content\n"));
                process_incoming_message(manager, &content, account).await;
            }
        }
        Err(err) => {
            crate::core::purple_error(account, 16, err.to_string());
        }
    }
    crate::core::purple_error(account, 0, String::from("Receiver has finished. Disconnected?"));
}
