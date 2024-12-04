use mime_sniffer::MimeTypeSniffer;

/*
 * Sends a text message to a contact identified by their uuid or to a group identified by its key.
 *
 * Taken from presage-cli
 */
pub async fn send<C: presage::store::Store + 'static>(
    manager: &mut presage::Manager<C, presage::manager::Registered>,
    recipient: crate::structs::Recipient,
    body: Option<String>,
    xfer: *const std::os::raw::c_void,
) -> Result<(), presage::Error<<C>::Error>> {
    let timestamp = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).expect("Time went backwards").as_millis() as u64;
    let mut data_message = presage::libsignal_service::content::DataMessage {
        timestamp: Some(timestamp),
        ..Default::default()
    };

    if xfer != std::ptr::null() {
        let path = crate::bridge::xfer_get_local_filename(xfer);
        let blob = std::fs::read(path.clone()).expect("Unable to read file.");
        let content_type = blob.sniff_mime_type().expect("Unable to guess content type.");
        let attachment = make_attachment(blob.clone(), content_type.to_string(), std::path::PathBuf::from(path));
        let upload_attachments_result = manager.upload_attachments(vec![attachment]).await?;
        let pointer = upload_attachments_result
            .first()
            .expect("At least one attachment pointer should be available")
            .as_ref()
            .expect("Failed to upload attachments"); // TODO: fail less hard if this happens
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
