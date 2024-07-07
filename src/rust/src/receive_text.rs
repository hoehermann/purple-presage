// TODO: rename to receive since it does also receive attachments

use futures::StreamExt; // for Stream.next()

/*
 * Prepares a received message's text for the front-end.
 *
 * Based on presage-cli's `print_message`.
 */
fn print_message<C: presage::store::Store>(
    manager: &presage::Manager<C, presage::manager::Registered>,
    content: &presage::libsignal_service::content::Content,
    account: *const std::os::raw::c_void,
) {
    println!("rust: print_message called…");
    let Ok(thread) = presage::store::Thread::try_from(content) else {
        println!("rust: failed to derive thread from content");
        return;
    };

    let format_data_message = |thread: &presage::store::Thread, data_message: &presage::libsignal_service::content::DataMessage| {
        match data_message {
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
            presage::libsignal_service::content::DataMessage {
                reaction:
                    Some(presage::proto::data_message::Reaction {
                        target_sent_timestamp: Some(timestamp),
                        emoji: Some(emoji),
                        ..
                    }),
                ..
            } => {
                let Ok(Some(message)) = manager.store().message(thread, *timestamp) else {
                    println!("rust: no message in {thread} sent at {timestamp}");
                    return None;
                };

                let presage::libsignal_service::content::ContentBody::DataMessage(presage::libsignal_service::content::DataMessage {
                    body: Some(body), ..
                }) = message.body
                else {
                    println!("rust: message reacted to has no body");
                    return None;
                };

                Some(format!("Reacted with {emoji} to message: \"{body}\""))
            }
            presage::libsignal_service::content::DataMessage {
                body: Some(body), ..
            } => Some(body.to_string()),
            c => {
                println!("rust: Empty data message {c:?}");
                // Note: flags: Some(4) with a timestamp (and a profile_key?) may indicate "message sent"
                // Some("message has been sent".to_string())
                None
            }
        }
    };

    let format_contact = |uuid| {
        manager
            .store()
            .contact_by_id(uuid)
            .ok()
            .flatten()
            .filter(|c| !c.name.is_empty())
            .map(|c| c.name)
            .unwrap_or_else(|| uuid.to_string())
    };
    let group_get_title = |key| manager.store().group(key).ok().flatten().map(|g| g.title).unwrap_or_else(|| "<missing group>".to_string());

    enum Msg<'a> {
        Received(&'a presage::store::Thread, String),
        Sent(&'a presage::store::Thread, String),
    }

    if let Some(msg) = match &content.body {
        presage::libsignal_service::content::ContentBody::NullMessage(_) => Some(Msg::Received(&thread, "Null message (for example deleted)".to_string())),
        presage::libsignal_service::content::ContentBody::DataMessage(data_message) => format_data_message(&thread, data_message).map(|body| Msg::Received(&thread, body)),
        presage::libsignal_service::content::ContentBody::SynchronizeMessage(presage::libsignal_service::content::SyncMessage {
            sent: Some(presage::proto::sync_message::Sent {
                message: Some(data_message),
                ..
            }),
            ..
        }) => format_data_message(&thread, data_message).map(|body| Msg::Sent(&thread, body)),
        presage::libsignal_service::content::ContentBody::CallMessage(_) => Some(Msg::Received(&thread, "is calling!".into())),
        // TODO: forward these properly
        presage::libsignal_service::content::ContentBody::TypingMessage(_) => None, //Some(Msg::Received(&thread, "is typing...".into())), // too annyoing for now. also does not differentiate between "started typing" and "stopped typing"
        presage::libsignal_service::content::ContentBody::ReceiptMessage(_) => None, //Some(Msg::Received(&thread, "received a message.".into())), // works, but too annyoing for now
        c => {
            println!("rust: unsupported message {c:?}");
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
                message.name = std::ffi::CString::new(format_contact(sender)).unwrap().into_raw();
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
                message.name = std::ffi::CString::new(format_contact(&content.metadata.sender.uuid)).unwrap().into_raw();
                message.group = std::ffi::CString::new(hex::encode(key)).unwrap().into_raw();
                message.title = std::ffi::CString::new(group_get_title(*key)).unwrap().into_raw();
                message.body = std::ffi::CString::new(body).unwrap().into_raw();
            }
            Msg::Sent(presage::store::Thread::Group(key), body) => {
                message.flags = 0x0001 | 0x10000; // PURPLE_MESSAGE_SEND | PURPLE_MESSAGE_REMOTE_SEND
                message.group = std::ffi::CString::new(hex::encode(key)).unwrap().into_raw();
                message.title = std::ffi::CString::new(group_get_title(*key)).unwrap().into_raw();
                message.body = std::ffi::CString::new(body).unwrap().into_raw();
            }
        };
        //println!("{who} in {group} wrote {body}");
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
    print_message(manager, content, account);

    if let presage::libsignal_service::content::ContentBody::DataMessage(presage::libsignal_service::content::DataMessage { attachments, .. }) = &content.body {
        for attachment_pointer in attachments {
            let mut message = crate::bridge::Presage::from_account(account);
            message.timestamp = content.metadata.timestamp;
            message.who = std::ffi::CString::new(content.metadata.sender.uuid.to_string()).unwrap().into_raw();

            let Ok(attachment_data) = manager.get_attachment(attachment_pointer).await else {
                message.flags = 0x0200; // PURPLE_MESSAGE_ERROR
                message.body = std::ffi::CString::new(String::from("Failed to fetch attachment.")).unwrap().into_raw();
                crate::bridge::append_message(&message);
                continue;
            };

            let extensions = mime_guess::get_mime_extensions_str(attachment_pointer.content_type.as_deref().unwrap_or("application/octet-stream"));
            let extension = extensions
                .and_then(|e| {
                    e.last() // using e.last here yields jpg instead of jfif, but it feels arbitrary
                })
                .unwrap_or(&"bin");
            /*
            let filename = attachment_pointer
                .file_name
                .clone()
                .unwrap_or_else(|| Local::now().format("%Y-%m-%d-%H-%M-%s").to_string());
             */
            let filename = match attachment_pointer.attachment_identifier.clone().unwrap() {
                presage::proto::attachment_pointer::AttachmentIdentifier::CdnId(id) => id.to_string(),
                presage::proto::attachment_pointer::AttachmentIdentifier::CdnKey(key) => key,
            };
            message.name = std::ffi::CString::new(format!("{filename}.{extension}")).unwrap().into_raw();
            let boxed_slice = attachment_data.into_boxed_slice();
            message.blobsize = boxed_slice.len() as u64; // TODO: blobsize should be a C type compatible with usize
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
    println!("rust: receive on separate thread begins…");
    let messages = manager.receive_messages(presage::manager::ReceivingMode::Forever).await;
    match messages {
        Ok(messages) => {
            println!("rust: receive got a message");
            futures::pin_mut!(messages);
            while let Some(content) = messages.next().await {
                // NOTE: This blocks until there is a message to be handled. Blocking forever seems to be by design.
                println!("rust: receive got a message's content");
                process_incoming_message(manager, &content, account).await;
            }
        }
        Err(err) => {
            crate::core::purple_error(account, 16, err.to_string());
        }
    }
    println!("rust: receive on separate thread finished.");
}
