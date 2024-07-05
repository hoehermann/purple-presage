/*
 * Sends a text message to a contact identified by their uuid or to a group identified by its key.
 *
 * Taken from presage-cli
 */
pub async fn send<C: presage::store::Store + 'static>(
    manager: &mut presage::Manager<C, presage::manager::Registered>,
    recipient: crate::structs::Recipient,
    message: &str,
) -> Result<(), presage::Error<<C>::Error>> {
    let timestamp = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).expect("Time went backwards").as_millis() as u64;
    // TODO: reduce redunancy be moving setting timestamp and body outside of recipient type switch
    match recipient {
        crate::structs::Recipient::Contact(uuid) => {
            let data_message = presage::libsignal_service::content::ContentBody::DataMessage(presage::libsignal_service::content::DataMessage {
                body: Some(message.to_string()),
                timestamp: Some(timestamp),
                ..Default::default()
            });
            manager.send_message(uuid, data_message, timestamp).await?;
        }
        crate::structs::Recipient::Group(master_key) => {
            let data_message = presage::libsignal_service::content::ContentBody::DataMessage(presage::libsignal_service::content::DataMessage {
                body: Some(message.to_string()),
                group_v2: Some(presage::proto::GroupContextV2 {
                    master_key: Some(master_key.to_vec()),
                    revision: Some(0),
                    ..Default::default()
                }),
                timestamp: Some(timestamp),
                ..Default::default()
            });
            manager.send_message_to_group(&master_key, data_message, timestamp).await?;
        }
    }

    Ok(())
}
