use futures::StreamExt; // for Stream.next()

/*
 * Prepares a received message's text for the front-end.
 * 
 * Based on presage-cli's `print_message`.
 */
fn print_message<C: presage::Store>(
    manager: &presage::Manager<C, presage::Registered>,
    content: &presage::prelude::Content,
    account: *const std::os::raw::c_void,
) {
    let Ok(thread) = presage::Thread::try_from(content) else {
        println!("rust: failed to derive thread from content");
        return;
    };
    let mut message = crate::bridge::Presage::from_account(account);

    let format_data_message = |thread: &presage::Thread, data_message: &presage::prelude::content::DataMessage| {
        match data_message {
            presage::prelude::content::DataMessage {
                quote: Some(presage::prelude::proto::data_message::Quote {
                    text: Some(quoted_text),
                    ..
                }),
                body: Some(body),
                ..
            } => Some(format!("Answer to message \"{quoted_text}\": {body}")),
            presage::prelude::content::DataMessage {
                reaction:
                    Some(presage::prelude::proto::data_message::Reaction {
                        target_sent_timestamp: Some(timestamp),
                        emoji: Some(emoji),
                        ..
                    }),
                ..
            } => {
                let Ok(Some(message)) = manager.message(thread, *timestamp) else {
                        println!("rust: no message in {thread} sent at {timestamp}");
                        return None;
                    };

                let presage::prelude::content::ContentBody::DataMessage(presage::prelude::DataMessage { body: Some(body), .. }) = message.body else {
                        println!("rust: message reacted to has no body");
                        return None;
                    };

                Some(format!("Reacted with {emoji} to message: \"{body}\""))
            }
            presage::prelude::content::DataMessage {
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

    /*
    let format_contact = |uuid| {
        manager
            .contact_by_id(uuid)
            .ok()
            .flatten()
            .filter(|c| !c.name.is_empty())
            .map(|c| format!("{}: {}", c.name, uuid))
            .unwrap_or_else(|| uuid.to_string())
    };
    let group_get_title = |key| {
        manager
            .group(key)
            .ok()
            .flatten()
            .map(|g| g.title)
            .unwrap_or_else(|| "<missing group>".to_string())
    };
    */

    enum Msg<'a> {
        Received(&'a presage::Thread, String),
        Sent(&'a presage::Thread, String),
    }

    if let Some(msg) = match &content.body {
        presage::prelude::content::ContentBody::NullMessage(_) => Some(Msg::Received(&thread, "Null message (for example deleted)".to_string())),
        presage::prelude::content::ContentBody::DataMessage(data_message) => format_data_message(&thread, data_message).map(|body| Msg::Received(&thread, body)),
        presage::prelude::content::ContentBody::SynchronizeMessage(presage::prelude::SyncMessage {
            sent: Some(presage::prelude::proto::sync_message::Sent {
                message: Some(data_message),
                ..
            }),
            ..
        }) => format_data_message(&thread, data_message).map(|body| Msg::Sent(&thread, body)),
        presage::prelude::content::ContentBody::CallMessage(_) => Some(Msg::Received(&thread, "is calling!".into())),
        // TODO: forward this as typing message
        //presage::prelude::content::ContentBody::TypingMessage(_) => Some(Msg::Received(&thread, "is typing...".into())),
        c => {
            println!("rust: unsupported message {c:?}");
            None
        }
    } {
        let (who, group, body, sent) = match msg {
            Msg::Received(presage::Thread::Contact(sender), body) => (sender.to_string(), String::from(""), body, false),
            Msg::Sent(presage::Thread::Contact(recipient), body) => (recipient.to_string(), String::from(""), body, true),
            Msg::Received(presage::Thread::Group(key), body) => {
                let group = hex::encode(key);
                (content.metadata.sender.uuid.to_string(), group, body, false)
            }
            Msg::Sent(presage::Thread::Group(key), body) => {
                let group = hex::encode(key);
                (String::from(""), group, body, true)
            }
        };

        println!("{who} in {group} wrote {body}");
        message.timestamp = content.metadata.timestamp;
        message.sent = if sent { 1 } else { 0 };
        if who != "" {
            message.who = std::ffi::CString::new(who).unwrap().into_raw();
        }
        if group != "" {
            message.group = std::ffi::CString::new(group).unwrap().into_raw();
        }
        message.body = std::ffi::CString::new(body).unwrap().into_raw();
        crate::bridge::append_message(&message);
    }
}

/*
 * Prepares a received message (text and attachments) for further processing.
 * 
 * Based on presage-cli's `process_incoming_message`.
 */
async fn process_incoming_message<C: presage::Store>(
    manager: &mut presage::Manager<C, presage::Registered>,
    content: &presage::prelude::Content,
    account: *const std::os::raw::c_void,
) {
    print_message(manager, content, account);

    /*
    let sender = content.metadata.sender.uuid;
    if let ContentBody::DataMessage(DataMessage { attachments, .. }) = &content.body {
        for attachment_pointer in attachments {
            let Ok(attachment_data) = manager.get_attachment(attachment_pointer).await else {
                log::warn!("failed to fetch attachment");
                continue;
            };

            let extensions = mime_guess::get_mime_extensions_str(
                attachment_pointer
                    .content_type
                    .as_deref()
                    .unwrap_or("application/octet-stream"),
            );
            let extension = extensions.and_then(|e| e.first()).unwrap_or(&"bin");
            let filename = attachment_pointer
                .file_name
                .clone()
                .unwrap_or_else(|| Local::now().format("%Y-%m-%d-%H-%M-%s").to_string());
            let file_path = attachments_tmp_dir.join(format!("presage-{filename}.{extension}",));
            match fs::write(&file_path, &attachment_data).await {
                Ok(_) => info!("saved attachment from {sender} to {}", file_path.display()),
                Err(error) => error!(
                    "failed to write attachment from {sender} to {}: {error}",
                    file_path.display()
                ),
            }
        }
    }
    */
}

/*
 * Receives messages from Signal servers.
 * 
 * Blocks forever.
 * 
 * Based on presage-cli's `receive`.
 */
pub async fn receive<C: presage::Store>(
    manager: &mut presage::Manager<C, presage::Registered>,
    account: *const std::os::raw::c_void,
) {
    let messages = manager.receive_messages().await;
    match messages {
        Ok(messages) => {
            futures::pin_mut!(messages);
            while let Some(content) = messages.next().await {
                process_incoming_message(manager, &content, account).await;
            }
        }
        Err(err) => {
            // TODO: forward error to front-end
            panic!("receive err {err}")
        }
    }
}
