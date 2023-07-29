pub async fn send<C: presage::Store + 'static>(
    msg: &str,
    uuid: &presage::prelude::Uuid,
    manager: &mut presage::Manager<C, presage::Registered>,
) -> Result<(), presage::Error<<C>::Error>> {
    let timestamp = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).expect("Time went backwards").as_millis() as u64;

    let message = presage::prelude::ContentBody::DataMessage(presage::prelude::DataMessage {
        body: Some(msg.to_string()),
        timestamp: Some(timestamp),
        ..Default::default()
    });

    manager.send_message(*uuid, message, timestamp).await?;
    Ok(())
}
