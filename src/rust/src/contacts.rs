/*
 * Reads all the contacts from the local store and forwards them to purple.
 *
 * The store is populated once during linking. Entries may be added and updated when receiving messages.
 */
pub async fn forward_contacts<C: presage::store::Store + 'static>(
    account: *mut crate::bridge_structs::PurpleAccount,
    manager: &mut presage::Manager<C, presage::manager::Registered>,
) {
    match manager.store().contacts().await {
        Err(err) => {
            crate::bridge::purple_debug(account, crate::bridge_structs::PURPLE_DEBUG_ERROR, format!("Unable to get contacts due to {err:?}\n"));
        }
        Ok(contacts) => {
            for presage::model::contacts::Contact {
                name,
                uuid,
                phone_number,
                ..
            } in contacts.flatten()
            {
                let message = crate::bridge::Message {
                    account: account,
                    who: Some(uuid.to_string()),
                    name: if name.is_empty() { None } else { Some(name) },
                    phone_number: phone_number.map(|pn| pn.to_string()),
                    ..Default::default()
                };
                crate::bridge::append_message(message);
            }
        }
    }
}

pub async fn get_group_members<C: presage::store::Store + 'static>(
    account: *mut crate::bridge_structs::PurpleAccount,
    manager: presage::Manager<C, presage::manager::Registered>,
    key: [u8; 32],
) -> Result<(), presage::Error<<C>::Error>> {
    match manager.store().group(key).await? {
        Some(group) => {
            let groups = vec![crate::bridge::Group::from_group(key, group)];
            crate::bridge::append_message(crate::bridge::Message {
                account: account,
                groups: groups,
                ..Default::default()
            });
        }
        None => {
            let key = hex::encode(key);
            crate::bridge::purple_debug(account, crate::bridge_structs::PURPLE_DEBUG_ERROR, format!("The group with key „{key}“ seems to be empty.\n"));
        }
    }
    Ok(())
}

pub async fn forward_groups<C: presage::store::Store + 'static>(
    account: *mut crate::bridge_structs::PurpleAccount,
    manager: &mut presage::Manager<C, presage::manager::Registered>,
) {
    match manager.store().groups().await {
        Err(err) => {
            crate::bridge::purple_debug(account, crate::bridge_structs::PURPLE_DEBUG_ERROR, format!("Unable to get groups due to {err:?}\n"));
        }
        Ok(groups) => {
            let groups: Vec<crate::bridge::Group> = groups.flatten().map(|(group_master_key, group)| crate::bridge::Group::from_group(group_master_key, group)).collect();
            crate::bridge::append_message(crate::bridge::Message {
                account: account,
                groups: groups,
                ..Default::default()
            });
        }
    }
}
