/*
 * Reads all the contacts from the local store and forwards them to purple.
 *
 * The store is populated once during linking. Entries may be added and updated when receiving messages.
 */
pub async fn forward_contacts<C: presage::store::Store + 'static>(
    account: *mut crate::bridge_structs::PurpleAccount,
    manager: &mut presage::Manager<C, presage::manager::Registered>,
) {
    // Collect first so the store() borrow is released before we call manager.retrieve_profile_*
    // (which needs &mut manager) inside the loop below.
    let contacts: Vec<presage::model::contacts::Contact> = match manager.store().contacts().await {
        Err(err) => {
            crate::bridge::purple_debug(account, crate::bridge_structs::PURPLE_DEBUG_ERROR, format!("Unable to get contacts due to {err:?}\n"));
            return;
        }
        Ok(contacts) => contacts.flatten().collect(),
    };

    for presage::model::contacts::Contact {
        name,
        uuid,
        phone_number,
        profile_key,
        ..
    } in contacts
    {
        // webOS: on a linked (secondary) device the address-book `name` synced from the primary
        // phone is frequently empty, so the buddy would show only its raw UUID. Fall back to the
        // contact's own Signal profile name (what they set for themselves) when we have a valid
        // profile key, so buddies get a human alias. phone_number is still forwarded separately.
        let mut display_name = name;
        if display_name.is_empty() && profile_key.len() == presage::libsignal_service::zkgroup::PROFILE_KEY_LEN {
            if let Ok(profilek) = profile_key.try_into() {
                let profile_key = presage::libsignal_service::prelude::ProfileKey::create(profilek);
                match manager.retrieve_profile_by_uuid(uuid, profile_key).await {
                    Ok(profile) => {
                        if let Some(profile_name) = profile.name {
                            display_name = profile_name.to_string();
                        }
                    }
                    Err(err) => crate::bridge::purple_debug(account, crate::bridge_structs::PURPLE_DEBUG_INFO, format!("No profile name for {uuid}: {err:?}\n")),
                }
            }
        }

        // Last resort: a phone number beats a raw UUID as the visible name, AND lets the webOS
        // Contacts app link this buddy to an address-book Person by number (which then shows the
        // real saved name, e.g. "Alan"). Without any number the buddy is an unlinkable bare UUID.
        let phone_number = phone_number.map(|pn| pn.to_string());
        if display_name.is_empty() {
            if let Some(pn) = &phone_number { display_name = pn.clone(); }
        }
        let message = crate::bridge::Message {
            account: account,
            who: Some(uuid.to_string()),
            name: if display_name.is_empty() { None } else { Some(display_name) },
            phone_number: phone_number,
            ..Default::default()
        };
        crate::bridge::append_message(message);
    }
}

/*
 * Resolve a display name for a single contact OFF the receive hot path.
 *
 * webOS: forward_contacts only names buddies that exist in the primary phone's Signal address
 * book. Contacts you have only exchanged messages with (never saved on the phone) are added as
 * bare-UUID buddies by the receive loop and stay nameless -- shown as a raw UUID, and never linked
 * to an address-book Person. This fills that gap: prefer the synced address-book name if the store
 * has one, else fall back to the contact's OWN Signal profile name (what they set for themselves).
 *
 * Runs as a fire-and-forget `spawn_local` task on a CLONED manager so it never blocks the receive
 * loop, and the profile fetch is bounded by a timeout. An earlier version did this inline in
 * process_incoming_message with an untimed fetch, which stalled the entire receive loop when the
 * fetch hung -- so no further messages arrived (see the note in receive.rs). Here the receive loop
 * keeps running regardless; the caller throttles to one attempt per UUID per session.
 */
pub async fn resolve_name_background<C: presage::store::Store + 'static>(
    account: *mut crate::bridge_structs::PurpleAccount,
    mut manager: presage::Manager<C, presage::manager::Registered>,
    uuid: presage::libsignal_service::prelude::Uuid,
    hint_profile_key: Option<Vec<u8>>,
) {
    // Look up the stored contact first (address-book name is authoritative when present). The
    // store() borrow is released at the end of this statement, before the &mut retrieve below.
    let stored = manager
        .store()
        .contact_by_id(&presage::libsignal_service::protocol::ServiceId::Aci(uuid.into()))
        .await
        .ok()
        .flatten();
    let mut display_name = String::new();
    let mut phone_number: Option<String> = None;
    let mut profile_key: Option<Vec<u8>> = hint_profile_key;
    if let Some(contact) = &stored {
        phone_number = contact.phone_number.as_ref().map(|pn| pn.to_string());
        if !contact.name.is_empty() {
            display_name = contact.name.clone();
        }
        if profile_key.is_none() && contact.profile_key.len() == presage::libsignal_service::zkgroup::PROFILE_KEY_LEN {
            profile_key = Some(contact.profile_key.clone());
        }
    }
    // Fall back to the contact's own profile name for un-named (unsaved) contacts.
    if display_name.is_empty() {
        if let Some(pk) = profile_key {
            if pk.len() == presage::libsignal_service::zkgroup::PROFILE_KEY_LEN {
                if let Ok(profilek) = pk.try_into() {
                    let profile_key = presage::libsignal_service::prelude::ProfileKey::create(profilek);
                    match tokio::time::timeout(std::time::Duration::from_secs(10), manager.retrieve_profile_by_uuid(uuid, profile_key)).await {
                        Ok(Ok(profile)) => {
                            if let Some(profile_name) = profile.name {
                                display_name = profile_name.to_string();
                            }
                        }
                        Ok(Err(err)) => crate::bridge::purple_debug(account, crate::bridge_structs::PURPLE_DEBUG_INFO, format!("background name: no profile for {uuid}: {err:?}\n")),
                        Err(_) => crate::bridge::purple_debug(account, crate::bridge_structs::PURPLE_DEBUG_INFO, format!("background name: profile fetch timed out for {uuid}\n")),
                    }
                }
            }
        }
    }
    // Last resort: fall back to the phone number rather than leaving the buddy as a bare UUID -
    // it's recognisable and lets webOS link the buddy to an address-book Person (which supplies
    // the real saved name). Matches forward_contacts.
    if display_name.is_empty() {
        if let Some(pn) = &phone_number { display_name = pn.clone(); }
    }
    if !display_name.is_empty() {
        crate::bridge::append_message(crate::bridge::Message {
            account: account,
            who: Some(uuid.to_string()),
            name: Some(display_name),
            phone_number: phone_number,
            ..Default::default()
        });
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

pub async fn get_profile<C: presage::store::Store + 'static>(
    account: *mut crate::bridge_structs::PurpleAccount,
    manager: &mut presage::Manager<C, presage::manager::Registered>,
    uuid: presage::libsignal_service::prelude::Uuid,
) -> Result<presage::model::contacts::Contact, Box<dyn std::error::Error>> {
    let contact = manager.store().contact_by_id(&presage::libsignal_service::protocol::ServiceId::Aci(uuid.into())).await?;
    let mut contact = contact.ok_or("No contact information available.".to_string())?;

    // we have a contact, try to update their profile
    match contact.profile_key.len() {
        0 => crate::bridge::purple_debug(account, crate::bridge_structs::PURPLE_DEBUG_INFO, format!("Missing profile key for {uuid}.\n")),
        presage::libsignal_service::zkgroup::PROFILE_KEY_LEN => {
            let profilek = contact.profile_key.clone().try_into().map_err(|_| "Invalid profile key although length has been checked.")?;
            let profile_key = presage::libsignal_service::prelude::ProfileKey::create(profilek);
            let profile = manager.retrieve_profile_by_uuid(uuid, profile_key).await?;
            crate::bridge::purple_debug(account, crate::bridge_structs::PURPLE_DEBUG_INFO, format!("Profile for {uuid}: {profile:?}\n"));
            // webOS: prefer the contact's own profile name when the synced address-book name is
            // empty, so the get_info popup (and the returned contact) carries a human name.
            if contact.name.is_empty() {
                if let Some(profile_name) = profile.name {
                    contact.name = profile_name.to_string();
                }
            }
        }
        l => crate::bridge::purple_debug(
            account,
            crate::bridge_structs::PURPLE_DEBUG_INFO,
            format!("Expected profile key length {}, got {l} for {uuid}.\n", presage::libsignal_service::zkgroup::PROFILE_KEY_LEN),
        ),
    }

    Ok(contact)
}
