async fn lookup_message_by_body_contains<S: presage::store::Store>(
    manager: &presage::Manager<S, presage::manager::Registered>,
    thread: &presage::store::Thread,
    pat: String,
) -> Option<presage::libsignal_service::content::Content> {
    if let Ok(now) = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH) {
        //print!("(xx:xx:xx) presage: now is {now:?}.“\n");
        match manager.store().messages(thread, 0..now.as_millis() as u64).await {
            Err(_) => None,
            Ok(messages) => {
                //print!("(xx:xx:xx) presage: messages exist for this thread.\n");
                messages
                    .filter(|result| {
                        result.as_ref().is_ok_and(|content| {
                            let body = &content.body;
                            match body {
                                presage::libsignal_service::content::ContentBody::DataMessage(data_message) => {
                                    let result = data_message.body().contains(&pat);
                                    //print!("(xx:xx:xx) presage: checking against „{body:?} → {result}“…\n");
                                    result
                                }
                                presage::libsignal_service::content::ContentBody::SynchronizeMessage(sync_message) => {
                                    let result = sync_message
                                        .sent
                                        .as_ref()
                                        .is_some_and(|sent| sent.message.as_ref().is_some_and(|data_message| data_message.body().contains(&pat)));
                                    //print!("(xx:xx:xx) presage: checking against „{body:?} → {result}“…\n");
                                    result
                                }
                                //presage::libsignal_service::content::ContentBody::EditMessage(edit_message) => … TODO
                                _ => false,
                            }
                        })
                    })
                    .last()
                    .and_then(|result| result.ok())
            }
        }
    } else {
        None
    }
}

fn extract_pat(input: &str) -> Option<&str> {
    if input.starts_with('@') {
        if let Some(colon_pos) = input.find(':') {
            return Some(&input[1..colon_pos]);
        }
    }
    None
}

/*
 * Sends a text message to a recipient.
 * 
 * Recipient may be a contact (identified by their uuid) or to a group (identified by its key).
 * 
 * Optionally also sends a file.
 *
 * Taken from presage-cli
 */
pub async fn send<C: presage::store::Store + 'static>(
    manager: &mut presage::Manager<C, presage::manager::Registered>,
    recipient: crate::structs::Recipient,
    body: Option<String>,
    xfer: *mut crate::bridge_structs::PurpleXfer,
) -> Result<(), anyhow::Error> {
    // -> Result<(), presage::Error<<C>::Error>>
    let timestamp = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).expect("Time went backwards").as_millis() as u64;
    let mut data_message = presage::libsignal_service::content::DataMessage {
        timestamp: Some(timestamp),
        ..Default::default()
    };

    if let Some(pat) = body.as_ref().and_then(|s| extract_pat(s.as_str())) {
        let thread = match recipient {
            crate::structs::Recipient::Contact(uuid) => presage::store::Thread::Contact(uuid),
            crate::structs::Recipient::Group(key) => presage::store::Thread::Group(key),
        };
        //print!("(xx:xx:xx) presage: Trying to Quote something with {pat:?}. Thread is {thread:?}.“\n");
        if let Some(quoted_message) = lookup_message_by_body_contains(manager, &thread, pat.to_string()).await {
            //print!("(xx:xx:xx) presage: Found message to quote: {quoted_message:#?}\n");
            let body = match &quoted_message.body {
                presage::libsignal_service::content::ContentBody::DataMessage(data_message) => data_message.body.clone(),
                presage::libsignal_service::content::ContentBody::SynchronizeMessage(sync_message) => {
                    sync_message.sent.as_ref().and_then(|sent| sent.message.as_ref().and_then(|data_message| data_message.body.clone()))
                }
                _ => None,
            };
            data_message.quote = Some(presage::proto::data_message::Quote {
                id: Some(quoted_message.metadata.timestamp),
                author_aci: Some(quoted_message.metadata.sender.raw_uuid().to_string()),
                text: body,
                attachments: vec![], // TODO
                body_ranges: vec![],
                r#type: Some(0), // type: NORMAL
            });
        }
    }

    if xfer != std::ptr::null_mut() {
        let path = crate::bridge::xfer_get_local_filename(xfer);
        let blob = std::fs::read(path.clone())?;
        let content_type = mime_sniffer::MimeTypeSniffer::sniff_mime_type(&blob).unwrap_or("*/*"); // NOTE: I saw this being used in Signal over application/octet-stream
        let attachment = make_attachment(blob.clone(), content_type.to_string(), std::path::PathBuf::from(path));
        let upload_attachments_result = manager.upload_attachments(vec![attachment]).await?;
        let pointer = upload_attachments_result.into_iter().next().ok_or(anyhow::anyhow!("Not a single attachment upload succeeded."))??;
        data_message.attachments.push(pointer.clone());
    }

    data_message.body = body;
    match recipient {
        crate::structs::Recipient::Contact(uuid) => {
            manager
                .send_message(
                    presage::libsignal_service::protocol::ServiceId::Aci(uuid.into()),
                    presage::libsignal_service::content::ContentBody::DataMessage(data_message),
                    timestamp,
                )
                .await?;
        }
        crate::structs::Recipient::Group(master_key) => {
            data_message.group_v2 = Some(presage::proto::GroupContextV2 {
                master_key: Some(master_key.to_vec()),
                revision: Some(0),
                ..Default::default()
            });
            manager
                .send_message_to_group(&master_key, presage::libsignal_service::content::ContentBody::DataMessage(data_message), timestamp)
                .await?;
        }
    }

    Ok(())
}

/*
 * Constructs the AttachmentSpec out of bytes
 *
 * Taken from flare, mostly
 */
pub fn make_attachment(
    bytes: Vec<u8>,
    content_type: String,
    path: std::path::PathBuf,
) -> (presage::libsignal_service::sender::AttachmentSpec, Vec<u8>) {
    (
        presage::libsignal_service::sender::AttachmentSpec {
            content_type: content_type,
            length: bytes.len(),
            file_name: path.file_name().map(|f| f.to_string_lossy().to_string()),
            preview: None,
            voice_note: None,
            borderless: None,
            width: None,  // TODO: find out if this is needed for images
            height: None, // TODO: find out if this is needed for images
            caption: None,
            blur_hash: None, // TODO: find out if this is needed for images
        },
        bytes,
    )
}
