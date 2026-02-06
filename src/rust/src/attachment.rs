pub async fn get_attachment<C: presage::store::Store + 'static>(
    account: *mut crate::bridge_structs::PurpleAccount,
    manager: presage::Manager<C, presage::manager::Registered>,
    attachment_pointer: presage::proto::AttachmentPointer,
    xfer: *const crate::bridge_structs::PurpleXfer,
) {
    let result = async {
        let filepath = crate::bridge::xfer_get_local_filename(xfer);
        let directory = std::path::Path::new(&filepath).parent()
            .ok_or("Unable to get download target directory.")?;
        std::fs::create_dir_all(directory)
            .map_err(|err| format!("Failed to create directory due to {err}"))?;
        let mut file = tokio::fs::File::create(&filepath).await
            .map_err(|err| format!("Failed to create file due to {err}"))?;
        let attachment_data = manager.get_attachment(&attachment_pointer).await
            .map_err(|err| format!("Failed to fetch attachment due to {err}"))?;
        tokio::io::AsyncWriteExt::write_all(&mut file, &attachment_data).await
            .map_err(|err| format!("Failed to write file due to {err}"))?;
        tokio::io::AsyncWriteExt::flush(&mut file).await
            .map_err(|err| format!("Failed to flush file due to {err}"))?;
        Ok::<(), String>(())
    }
    .await;

    match result {
        Ok(_) => crate::bridge::append_message(crate::bridge::Message {
            account,
            xfer,
            mimetype: attachment_pointer.content_type,
            ..Default::default()
        }),
        Err(err) => crate::bridge::append_message(crate::bridge::Message {
            account,
            xfer,
            flags: crate::bridge_structs::PurpleMessageFlags::PURPLE_MESSAGE_ERROR,
            body: Some(err),
            ..Default::default()
        }),
    }
}
