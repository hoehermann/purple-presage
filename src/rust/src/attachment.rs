use tokio::io::AsyncWriteExt;

pub async fn get_attachment<C: presage::store::Store + 'static>(
    account: *mut crate::bridge_structs::PurpleAccount,
    manager: presage::Manager<C, presage::manager::Registered>,
    filepath: String,
    attachment_pointer: presage::proto::AttachmentPointer,
) {
    if let Ok(attachment_data) = manager.get_attachment(&attachment_pointer).await {
        match tokio::fs::File::create(filepath).await {
            Ok(mut file) => {
                match file.write_all(&attachment_data).await {
                    Ok(_) => {
                        //crate::bridge::append_message(message.name(filename).attachment(attachment_data));
                        // TODO: indicate success
                    },
                    Err(err) => {
                        crate::bridge::append_message(crate::bridge::Message {
                            account: account,
                            // TODO: reference to xfer
                            body: Some(format!("Failed to write file due to {}", err.to_string())),
                            flags: crate::bridge_structs::PurpleMessageFlags::PURPLE_MESSAGE_ERROR,
                            ..Default::default()
                        });
                    },
                }
            },
            Err(err) => {
                crate::bridge::append_message(crate::bridge::Message {
                    account: account,
                    // TODO: reference to xfer
                    body: Some(format!("Failed to create file due to {}", err.to_string())),
                    flags: crate::bridge_structs::PurpleMessageFlags::PURPLE_MESSAGE_ERROR,
                    ..Default::default()
                });
            },
        }
    } else {
        crate::bridge::append_message(crate::bridge::Message {
            account: account,
            // TODO: reference to xfer
            body: Some("Failed to fetch attachment.".to_string()),
            flags: crate::bridge_structs::PurpleMessageFlags::PURPLE_MESSAGE_ERROR,
            ..Default::default()
        });
    };


}