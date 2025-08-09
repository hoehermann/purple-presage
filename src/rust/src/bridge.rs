/*
 * A rust representation of the C Message struct.
 *
 * The idea is that we have rust types for safety until the very moment we transfer the data to the C part.
 */
#[derive(Clone, Debug)]
pub struct Message {
    pub account: *mut crate::bridge_structs::PurpleAccount,
    pub tx_ptr: *const tokio::sync::mpsc::Sender<crate::structs::Cmd>,
    pub qrcode: Option<String>,
    pub uuid: Option<String>,
    pub debug: i32,
    pub error: i32,
    pub connected: i32,
    pub timestamp: Option<u64>,
    pub flags: crate::bridge_structs::PurpleMessageFlags,
    pub who: Option<String>,
    pub name: Option<String>,
    pub phone_number: Option<String>,
    pub group: Option<String>,
    pub body: Option<String>,
    pub attachment_pointer: Option<presage::proto::AttachmentPointer>,
    pub hash: Option<String>,
    pub filename: Option<String>,
    pub extension: Option<String>,
    pub groups: Vec<crate::bridge::Group>,
    pub xfer: *const crate::bridge_structs::PurpleXfer,
    pub thread: Option<presage::store::Thread>,
}
#[derive(Clone, Debug)]
pub struct Group {
    pub key: String,
    pub title: String,
    pub description: String,
    pub revision: u32,
    pub members: Vec<String>,
}
// presage::model::groups::Group does not implement the clone trait, so we have this domain-specific variant
impl Group {
    pub fn from_group(
        key: [u8; 32],
        group: presage::model::groups::Group,
    ) -> Self {
        Self {
            key: hex::encode(key),
            title: group.title,
            description: group.description.unwrap_or_default(),
            revision: group.revision,
            members: group.members.into_iter().map(|member| member.uuid.to_string()).collect(),
            // `avatar`, `disappearing_messages_timer`, `access_control`, `pending_members`, `requesting_members`, `invite_link_password`
        }
    }
}
impl Default for Message {
    fn default() -> Self {
        Message {
            thread: None,
            account: std::ptr::null_mut(),
            timestamp: None,
            flags: crate::bridge_structs::PurpleMessageFlags::default(),
            who: None,
            name: None,
            group: None,
            body: None,
            attachment_pointer: None,
            hash: None,
            filename: None,
            extension: None,
            phone_number: None,
            error: -1,
            debug: -1,
            tx_ptr: std::ptr::null_mut(),
            connected: 0,
            uuid: None,
            groups: Vec::new(),
            qrcode: None,
            xfer: std::ptr::null_mut(),
        }
    }
}
impl Message {
    pub fn body(
        mut self,
        body: String,
    ) -> Self {
        self.body = Some(body);
        self
    }
    pub fn flags(
        mut self,
        flags: crate::bridge_structs::PurpleMessageFlags,
    ) -> Self {
        self.flags = flags;
        self
    }
}

extern "C" {
    // this is implemented by bridge.c
    fn presage_append_message(message: *const crate::bridge_structs::Message);

    // this is implemented by libpurple's ft.c
    // TODO: automatically generate declaration from ft.h
    fn purple_xfer_get_local_filename(xfer: *const crate::bridge_structs::PurpleXfer) -> *const std::os::raw::c_char;
}

// I want to forward a Vec of groups to the C part, but the rust-allocated C-compatible CStrings must live somewhere, so we have this intermediate type
struct CGroup {
    key: Option<std::ffi::CString>,
    title: Option<std::ffi::CString>,
    description: Option<std::ffi::CString>,
    c_members: Vec<*const std::os::raw::c_char>,
    members: Vec<std::ffi::CString>,
}

pub fn append_message(message: Message) {
    //print!("(xx:xx:xx) presage: append_message {message:#?}\n");
    let to_cstring = |s: Option<String>| -> Option<std::ffi::CString> { s.map_or(None, |s| std::ffi::CString::new(s).ok()) };
    let get_cstring_ptr = |s: &Option<std::ffi::CString>| {
        if let Some(ss) = s {
            return ss.as_ptr();
        }
        return std::ptr::null();
    };

    // let the CStrings live here
    let qrcode = to_cstring(message.qrcode);
    let uuid = to_cstring(message.uuid);
    let who = to_cstring(message.who);
    let name = to_cstring(message.name);
    let phone_number = to_cstring(message.phone_number);
    let group = to_cstring(message.group);
    let body = to_cstring(message.body);
    let attachment_size = message.attachment_pointer.as_ref().map_or(0, |a| a.size());
    let hash = to_cstring(message.hash);
    let filename = to_cstring(message.filename);
    let extension = to_cstring(message.extension);
    let groups_length = message.groups.len();
    // create a CString for every field for every CGroup
    let groups: Vec<CGroup> = message
        .groups
        .iter()
        .map(|g| {
            let mut c_group = CGroup {
                key: std::ffi::CString::new(g.key.clone()).ok(),
                title: std::ffi::CString::new(g.title.clone()).ok(),
                description: std::ffi::CString::new(g.description.clone()).ok(),
                members: g.members.iter().map(|m| std::ffi::CString::new(m.clone()).unwrap()).collect(),
                c_members: [].to_vec(),
            };
            // hava a C-compatible pointer to the vector of members
            c_group.c_members = c_group.members.iter().map(|cm| cm.as_ptr()).collect();
            c_group
        })
        .collect();
    // hava a C-compatible pointer to the vector of groups
    let c_groups: Vec<crate::bridge_structs::Group> = groups
        .iter()
        .zip(message.groups)
        .map(|(cg,rg)| crate::bridge_structs::Group {
            key: get_cstring_ptr(&cg.key),
            title: get_cstring_ptr(&cg.title),
            description: get_cstring_ptr(&cg.description),
            revision: rg.revision,
            members: cg.c_members.as_ptr(),
            population: cg.c_members.len(),
        })
        .collect();

    // wrap all data into one struct
    let c_message = crate::bridge_structs::Message {
        account: message.account,
        tx_ptr: message.tx_ptr as crate::bridge_structs::RustChannelPtr,
        qrcode: get_cstring_ptr(&qrcode),
        uuid: get_cstring_ptr(&uuid),
        debug: message.debug,
        error: message.error,
        connected: message.connected,
        attachment_pointer_box: message.attachment_pointer.map_or(std::ptr::null(), |a| Box::into_raw(Box::new(a)) as *const std::os::raw::c_void),
        hash: get_cstring_ptr(&hash),
        filename: get_cstring_ptr(&filename),
        extension: get_cstring_ptr(&extension),
        timestamp: message.timestamp.unwrap_or(0),
        flags: message.flags,
        who: get_cstring_ptr(&who),
        name: get_cstring_ptr(&name),
        phone_number: get_cstring_ptr(&phone_number),
        group: get_cstring_ptr(&group),
        body: get_cstring_ptr(&body),
        attachment_size: attachment_size,
        groups: c_groups.as_ptr(),
        groups_length: groups_length,
        xfer: message.xfer,
    };
    //print!("(xx:xx:xx) presage: c_message.groups is at {0:p}\n", c_message.groups);
    unsafe {
        presage_append_message(&c_message);
    }
}

// convenience function for calling purple_error on the main thread
pub fn purple_error(
    account: *mut crate::bridge_structs::PurpleAccount,
    level: crate::bridge_structs::PurpleConnectionError,
    msg: String,
) {
    append_message(Message {
        account: account,
        error: level,
        body: Some(msg),
        ..Default::default()
    });
}

// convenience function for calling purple_debug on the main thread
pub fn purple_debug(
    account: *mut crate::bridge_structs::PurpleAccount,
    level: crate::bridge_structs::PurpleDebugLevel,
    msg: String,
) {
    append_message(Message {
        account: account,
        debug: level,
        body: Some(msg),
        ..Default::default()
    });
}

// wrapper around unsafe purple_xfer_get_local_filename
pub fn xfer_get_local_filename(xfer: *const crate::bridge_structs::PurpleXfer) -> String {
    unsafe {
        return std::ffi::CStr::from_ptr(purple_xfer_get_local_filename(xfer)).to_str().unwrap().to_owned();
    }
}

/*
 * This library has no main function to annotate with `#[tokio::main]`, but needs a run-time.
 * This function creates a tokio runtime and boxes it so the runtime can live in the front-end.
 *
 * The approach is described at https://tokio.rs/tokio/topics/bridging.
 * https://stackoverflow.com/questions/66196972/ and https://stackoverflow.com/questions/64658556/ are helpful.
 */
#[no_mangle]
pub extern "C" fn presage_rust_init() -> *mut tokio::runtime::Runtime {
    let runtime = tokio::runtime::Builder::new_multi_thread().thread_name("presage Tokio").worker_threads(1).enable_all().build().unwrap();
    let runtime_box = Box::new(runtime);
    Box::into_raw(runtime_box)
}

#[no_mangle]
pub extern "C" fn presage_rust_destroy(runtime: *mut tokio::runtime::Runtime) {
    unsafe {
        drop(Box::from_raw(runtime));
    }
}

/*
 * Around the core's main function.
 *
 * This blocks until the rust main function finishes.
 */
#[no_mangle]
pub unsafe extern "C" fn presage_rust_main(
    account: *mut crate::bridge_structs::PurpleAccount,
    rt: *mut tokio::runtime::Runtime,
    c_store_path: *const std::os::raw::c_char,
) {
    let store_path = std::ffi::CStr::from_ptr(c_store_path).to_str().unwrap().to_owned();

    // create a channel for asynchronous communication of commands c → rust
    let (tx, rx) = tokio::sync::mpsc::channel(32);
    // pass the pointer to the channel to the C part
    // this should be safe as tx lives here and the runtime blocks here, too
    append_message(Message {
        account: account,
        tx_ptr: &tx as *const tokio::sync::mpsc::Sender<crate::structs::Cmd>,
        ..Default::default()
    }); // let front-end know how to reach us

    // now execute the actual program
    let runtime = rt.as_ref().unwrap();
    runtime.block_on(crate::core::main(store_path, None, rx, account));
    purple_debug(account, crate::bridge_structs::PURPLE_DEBUG_INFO, String::from("rust runtime finishes now…\n"));
}
