pub async fn get_attachment<C: presage::store::Store + 'static>(
    account: *mut crate::bridge_structs::PurpleAccount,
    manager: presage::Manager<C, presage::manager::Registered>,
    attachment_pointer: presage::proto::AttachmentPointer,
    xfer: *const crate::bridge_structs::PurpleXfer,
) {
    //print!("(xx:xx:xx) presage: rust get_attachment(…)…\n");
    let filepath = crate::bridge::xfer_get_local_filename(xfer);
    //print!("(xx:xx:xx) presage: rust get_attachment(…) filepath is „{filepath}“.\n");
    std::path::Path::new(&filepath).parent().and_then(|directory| std::fs::create_dir_all(directory).ok()); // TODO: error handling?
    match tokio::fs::File::create(filepath).await {
        Ok(mut file) => {
            if let Ok(attachment_data) = manager.get_attachment(&attachment_pointer).await {
                match tokio::io::AsyncWriteExt::write_all(&mut file, &attachment_data).await {
                    Ok(_) => {
                        //print!("(xx:xx:xx) presage: rust get_attachment(…) write_all Ok\n");
                        crate::bridge::append_message(crate::bridge::Message {
                            account: account,
                            xfer: xfer,
                            ..Default::default() // implies default for message flag which is not an error
                        });
                    }
                    Err(err) => {
                        //print!("(xx:xx:xx) presage: rust get_attachment(…) Failed to write file due to {}\n", err.to_string());
                        crate::bridge::append_message(crate::bridge::Message {
                            account: account,
                            xfer: xfer,
                            flags: crate::bridge_structs::PurpleMessageFlags::PURPLE_MESSAGE_ERROR,
                            body: Some(format!("Failed to write file due to {}", err.to_string())),
                            ..Default::default()
                        });
                    }
                }
            } else {
                crate::bridge::append_message(crate::bridge::Message {
                    account: account,
                    xfer: xfer,
                    body: Some("Failed to fetch attachment.".to_string()),
                    flags: crate::bridge_structs::PurpleMessageFlags::PURPLE_MESSAGE_ERROR,
                    ..Default::default()
                });
            };
        }
        Err(err) => {
            //print!("(xx:xx:xx) presage: rust get_attachment(…) Failed to create file due to {}\n", err.to_string());
            crate::bridge::append_message(crate::bridge::Message {
                account: account,
                xfer: xfer,
                flags: crate::bridge_structs::PurpleMessageFlags::PURPLE_MESSAGE_ERROR,
                body: Some(format!("Failed to create file due to {}", err.to_string())),
                ..Default::default()
            });
        }
    }
}
