pub async fn get_attachment<C: presage::store::Store + 'static>(
    account: *mut crate::bridge_structs::PurpleAccount,
    manager: presage::Manager<C, presage::manager::Registered>,
    attachment_pointer: presage::proto::AttachmentPointer,
    xfer: *const crate::bridge_structs::PurpleXfer,
) {
    let filepath = crate::bridge::xfer_get_local_filename(xfer);
    std::path::Path::new(&filepath).parent().and_then(|directory| std::fs::create_dir_all(directory).ok()); // TODO: error handling?
    match tokio::fs::File::create(filepath).await {
        Ok(mut file) => {
            match manager.get_attachment(&attachment_pointer).await {
                Ok(attachment_data) => match tokio::io::AsyncWriteExt::write_all(&mut file, &attachment_data).await {
                    Ok(_) => match tokio::io::AsyncWriteExt::flush(&mut file).await {
                        Ok(_) => crate::bridge::append_message(crate::bridge::Message {
                            account: account,
                            xfer: xfer,
                            mimetype: attachment_pointer.content_type,
                            ..Default::default() // implies default for message flag which is not an error
                        }),
                        Err(err) => crate::bridge::append_message(crate::bridge::Message {
                            account: account,
                            xfer: xfer,
                            flags: crate::bridge_structs::PurpleMessageFlags::PURPLE_MESSAGE_ERROR,
                            body: Some(format!("Failed to flush file due to {err}")),
                            ..Default::default()
                        }),
                    },
                    Err(err) => crate::bridge::append_message(crate::bridge::Message {
                        account: account,
                        xfer: xfer,
                        flags: crate::bridge_structs::PurpleMessageFlags::PURPLE_MESSAGE_ERROR,
                        body: Some(format!("Failed to write file due to {err}")),
                        ..Default::default()
                    }),
                },
                Err(err) => crate::bridge::append_message(crate::bridge::Message {
                    account: account,
                    xfer: xfer,
                    body: Some(format!("Failed to fetch attachment due to {err}.")),
                    flags: crate::bridge_structs::PurpleMessageFlags::PURPLE_MESSAGE_ERROR,
                    ..Default::default()
                }),
            };
        }
        Err(err) => {
            crate::bridge::append_message(crate::bridge::Message {
                account: account,
                xfer: xfer,
                flags: crate::bridge_structs::PurpleMessageFlags::PURPLE_MESSAGE_ERROR,
                body: Some(format!("Failed to create file due to {err}")),
                ..Default::default()
            });
        }
    }
}
