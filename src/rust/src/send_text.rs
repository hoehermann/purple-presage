/*
 * Sends a text message to a contact identified by their uuid.
 * 
 * Taken from presage-cli
 */
pub async fn send<C: presage::store::Store + 'static>(
    message: &str,
    uuid: &presage::libsignal_service::prelude::Uuid,
    manager: &mut presage::Manager<C, presage::manager::Registered>,
) -> Result<(), presage::Error<<C>::Error>> {
    let timestamp = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).expect("Time went backwards").as_millis() as u64;

    let message = presage::libsignal_service::content::ContentBody::DataMessage(presage::libsignal_service::content::DataMessage {
        body: Some(message.to_string()),
        timestamp: Some(timestamp),
        ..Default::default()
    });

    manager.send_message(*uuid, message, timestamp).await?;
    Ok(())
}
