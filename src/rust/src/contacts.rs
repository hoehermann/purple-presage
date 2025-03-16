pub async fn get_contacts<C: presage::store::Store + 'static>(
    account: *mut crate::bridge_structs::PurpleAccount,
    manager: &mut presage::Manager<C, presage::manager::Registered>,
) {
    match manager.store().contacts().await {
        Err(err) => {
            crate::bridge::purple_debug(account, 4, format!("Unable to get contacts due to {err:?}\n"));
        }
        Ok(contacts) => {
            for presage::model::contacts::Contact {
                name,
                uuid,
                phone_number,
                ..
            } in contacts.flatten()
            {
                let mut message = crate::bridge_structs::Message::from_account(account);
                message.who = std::ffi::CString::new(uuid.to_string()).unwrap().into_raw();
                message.name = if name != "" { std::ffi::CString::new(name).unwrap().into_raw() } else { std::ptr::null_mut() };
                message.phone_number = phone_number.map_or(std::ptr::null_mut(), |pn| std::ffi::CString::new(pn.to_string()).unwrap().into_raw());
                crate::bridge::append_message(&message);
            }
        }
    }
}

pub async fn get_group_members<C: presage::store::Store + 'static>(
    account: *mut crate::bridge_structs::PurpleAccount,
    manager: Option<presage::Manager<C, presage::manager::Registered>>,
    key: [u8; 32],
) -> Result<presage::Manager<C, presage::manager::Registered>, presage::Error<<C>::Error>> {
    let manager = manager.expect("manager must be loaded");
    match manager.store().group(key).await? {
        Some(group) => {
            let mut message = crate::bridge_structs::Message::from_account(account);
            let uuid_strings = group.members.into_iter().map(|member| member.uuid.to_string());
            let uuid_c_strings: Vec<*mut std::os::raw::c_char> = uuid_strings.map(|u| std::ffi::CString::new(u).unwrap().into_raw()).collect();
            let boxed_uuid_c_strings = uuid_c_strings.into_boxed_slice();
            let groups = vec![crate::bridge_structs::Group {
                key: std::ffi::CString::new(hex::encode(key)).unwrap().into_raw(),
                title: std::ffi::CString::new(group.title).unwrap().into_raw(),
                description: std::ffi::CString::new(group.description.unwrap_or("".to_string())).unwrap().into_raw(),
                revision: group.revision,
                population: boxed_uuid_c_strings.len(),
                members: Box::into_raw(boxed_uuid_c_strings) as *mut *mut std::os::raw::c_char,
            }];
            message.size = 1;
            message.groups = Box::into_raw(groups.into_boxed_slice()) as *mut crate::bridge_structs::Group;
            crate::bridge::append_message(&message);
        }
        None => {
            // TODO
        }
    }
    Ok(manager)
}

pub async fn get_groups<C: presage::store::Store + 'static>(
    account: *mut crate::bridge_structs::PurpleAccount,
    manager: &mut presage::Manager<C, presage::manager::Registered>,
) {
    match manager.store().groups().await {
        Err(err) => {
            crate::bridge::purple_debug(account, crate::bridge_structs::PURPLE_DEBUG_ERROR, format!("Unable to get groups due to {err:?}\n"));
        }
        Ok(groups) => {
            let groups: Vec<crate::bridge_structs::Group> = groups
                .flatten()
                .map(
                    |(
                        group_master_key,
                        presage::model::groups::Group {
                            title,
                            description,
                            revision,
                            members,
                            ..
                        },
                        // `avatar`, `disappearing_messages_timer`, `access_control`, `pending_members`, `requesting_members`, `invite_link_password`
                    )| {
                        let key = hex::encode(group_master_key);
                        crate::bridge_structs::Group {
                            key: std::ffi::CString::new(key).unwrap().into_raw(),
                            title: std::ffi::CString::new(title).unwrap().into_raw(),
                            description: std::ffi::CString::new(description.unwrap_or("".to_string())).unwrap().into_raw(),
                            revision: revision,
                            population: members.len(),
                            members: std::ptr::null_mut(),
                        }
                    },
                )
                .collect();
            let mut message = crate::bridge_structs::Message::from_account(account);
            message.size = groups.len();
            message.groups = Box::into_raw(groups.into_boxed_slice()) as *mut crate::bridge_structs::Group;
            crate::bridge::append_message(&message);
        }
    }
}
