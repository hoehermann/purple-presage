/*
 * Looks up the title of a group identified by its group master key.
 *
 * Adapted from presage-cli.
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
                // TODO: also return body_ranges
            } else {
                None
            }
        }
    }
}

/*
 * Turns a DataMessage into a string for presentation via libpurple.
 *
 * Adapted from presage-cli.
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
            quote:
                Some(presage::proto::data_message::Quote {
                    text: Some(quoted_text),
                    body_ranges: quoted_text_ranges,
                    ..
                }),
            body: Some(body),
            body_ranges,
            ..
        } => {
            let quote = resolve_mentions(quoted_text, quoted_text_ranges);
            let firstline = quote.split("\n").next().unwrap_or("<message body missing>");
            // TODO: add ellipsis if quoted_text contains more than one line
            let body = resolve_mentions(body, body_ranges);
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
        presage::libsignal_service::content::DataMessage {
            body: Some(body),
            body_ranges,
            ..
        } => Some(resolve_mentions(body, body_ranges)),
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

/*
 * Resolve mentions by Object Replacement Character with UUIDs.
 */
// TODO: forward body ranges and let front-end take care of resolving the UUIDs
// NOTE: keep an eye on PurpleMarkupSpan documented at https://issues.imfreedom.org/issue/PIDGIN-17842
fn resolve_mentions(
    body: &String,
    body_ranges: &Vec<presage::proto::BodyRange>,
) -> String {
    let mut body_ranges_iter = body_ranges.into_iter();
    body.chars()
        .map(|c| {
            if c == '￼' {
                if let Some(presage::proto::BodyRange {
                    associated_value: Some(presage::proto::body_range::AssociatedValue::MentionAci(mention_aci)),
                    ..
                }) = body_ranges_iter.next()
                {
                    // NOTE: This relies on mentions being sorted. This may or may not always be the case.
                    format!("@{mention_aci}")
                } else {
                    c.to_string()
                }
            } else {
                c.to_string()
            }
        })
        .collect()
}

async fn process_attachments<C: presage::store::Store>(
    manager: &mut presage::Manager<C, presage::manager::Registered>,
    message: crate::bridge::Message,
    attachments: &Vec<presage::proto::AttachmentPointer>,
) {
    let account = message.account;
    for attachment_pointer in attachments {
        let Ok(attachment_data) = manager.get_attachment(attachment_pointer).await else {
            crate::bridge::append_message(
                message
                    .clone()
                    .body("Failed to fetch attachment.".to_string())
                    .flags(crate::bridge_structs::PurpleMessageFlags::PURPLE_MESSAGE_ERROR),
            );
            continue;
        };

        match attachment_pointer.content_type.as_deref() {
            None => {
                crate::bridge::purple_debug(account, crate::bridge_structs::PURPLE_DEBUG_ERROR, format!("Received attachment without content type.\n"));
            }
            Some("text/x-signal-plain") => {
                // strip trailing null bytes, thanks to https://stackoverflow.com/questions/49406517/how-to-remove-trailing-null-characters-from-string#comment139692696_49406848
                // TODO: check if stripping the trailing null byte is still necessary now that https://github.com/whisperfish/presage/commit/ab8b3a8 is live
                match String::from_utf8(attachment_data) {
                    Ok(padded) => {
                        let body = padded.trim_end_matches(char::from(0));
                        // TODO: this should be routed through the function that usually handles the text messages
                        crate::bridge::append_message(message.clone().body(body.to_owned()));
                    }
                    Err(err) => {
                        crate::bridge::append_message(
                            message
                                .clone()
                                .body(format!("Failed to fetch long text message due to {err}"))
                                .flags(crate::bridge_structs::PurpleMessageFlags::PURPLE_MESSAGE_ERROR),
                        );
                    }
                }
            }
            Some(mimetype) => {
                let extension = match mimetype {
                    // use the most poplular default for some common mimetypes
                    "image/jpeg" => "jpg",
                    "image/png" => "png",
                    "video/mp4" => "mp4",
                    mimetype => {
                        let extensions = mime_guess::get_mime_extensions_str(mimetype);
                        extensions.and_then(|e| e.first()).unwrap_or(&"bin")
                    }
                };
                // TODO: have a user-configurable template for generating the file-name
                // NOTE: for some conversations, all image come with the same filename
                let hash = match attachment_pointer.attachment_identifier.clone().unwrap() {
                    presage::proto::attachment_pointer::AttachmentIdentifier::CdnId(id) => id.to_string(),
                    presage::proto::attachment_pointer::AttachmentIdentifier::CdnKey(key) => key,
                };
                let suffix = attachment_pointer.file_name.clone().unwrap_or_else(|| format!(".{extension}"));
                let filename = hash + &suffix;
                crate::bridge::append_message(message.clone().name(filename).attachment(attachment_data));
            }
        }
    }
}

async fn process_data_message<C: presage::store::Store>(
    manager: &mut presage::Manager<C, presage::manager::Registered>,
    message: crate::bridge::Message,
    data_message: &presage::proto::DataMessage,
) -> Option<String> {
    process_attachments(manager, message.clone(), &data_message.attachments).await;
    format_data_message(manager, message.account, &message.thread.unwrap(), data_message).await
}

async fn process_sent_message<C: presage::store::Store>(
    manager: &mut presage::Manager<C, presage::manager::Registered>,
    message: crate::bridge::Message,
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
        crate::bridge::append_message(message.clone().body(body));
    }
}

async fn process_sync_message<C: presage::store::Store>(
    manager: &mut presage::Manager<C, presage::manager::Registered>,
    message: crate::bridge::Message,
    sync_message: &presage::proto::SyncMessage,
) {
    // TODO: explicitly ignore SynchronizeMessage(SyncMessage { sent: None, contacts: None, request: None, read: [], blocked: None, verified: None, configuration: None, padding: Some([…]), …, delete_for_me: Some(DeleteForMe { message_deletes: [MessageDeletes { conversation: Some(ConversationIdentifier { identifier: Some(ThreadServiceId("REDACTED")) }), messages: [AddressableMessage { sent_timestamp: Some(1674147919685), author: Some(AuthorServiceId("REDACTED")) }] }], conversation_deletes: [], local_only_conversation_deletes: [], attachment_deletes: [] }) })
    if let Some(sent) = &sync_message.sent {
        process_sent_message(manager, message, sent).await;
    }
}

async fn process_received_message<C: presage::store::Store>(
    manager: &mut presage::Manager<C, presage::manager::Registered>,
    message: crate::bridge::Message,
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
        let mut message = message.clone();
        message.body = Some(body);
        crate::bridge::append_message(message);
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
    // TODO: check where thread is actually needed and look it up conditionally?
    let Ok(thread) = presage::store::Thread::try_from(content) else {
        crate::bridge::purple_error(account, crate::bridge_structs::PURPLE_CONNECTION_ERROR_OTHER_ERROR, String::from("failed to find conversation"));
        return;
    };
    let mut message = crate::bridge::Message {
        account: account,
        ..Default::default()
    };
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
    message.thread = Some(thread);
    message.timestamp = Some(content.metadata.timestamp);

    match &content.body {
        presage::libsignal_service::content::ContentBody::SynchronizeMessage(sync_message) => process_sync_message(manager, message, sync_message).await,
        _ => {
            message.flags = crate::bridge_structs::PurpleMessageFlags::PURPLE_MESSAGE_RECV;
            process_received_message(manager, message, &content.body).await
        }
    }
}

pub async fn handle_received<S: presage::store::Store>(
    manager: &mut presage::Manager<S, presage::manager::Registered>,
    account: *mut crate::bridge_structs::PurpleAccount,
    received: presage::model::messages::Received,
) {
    match received {
        presage::model::messages::Received::QueueEmpty => {
            // this happens once after all old messages have been received and processed
            crate::bridge::purple_debug(account, crate::bridge_structs::PURPLE_DEBUG_INFO, format!("finished catching up.\n"));

            // now that the initial sync has completed, the account can be regarded as "connected" since it is ready to send messages
            // NOTE: we already told the front-end the account was "connected" earlier because some of libpurple's blist functions do not work on offline accounts
            crate::bridge::append_message(crate::bridge::Message {
                account: account,
                connected: 1,
                ..Default::default()
            });
        }
        presage::model::messages::Received::Contacts => {
            // this happens in response to manager.request_contacts()
            crate::bridge::purple_debug(account, crate::bridge_structs::PURPLE_DEBUG_INFO, format!("received contacts\n"));
            crate::contacts::forward_contacts(account, manager).await;
            crate::contacts::forward_groups(account, manager).await; // TODO: find out how to actually request list of groups
        }
        presage::model::messages::Received::Content(content) => process_incoming_message(manager, &content, account).await,
    }
}
