use crate::bridge_structs::PurpleMessageFlags;

impl crate::bridge_structs::Message {
    pub fn from_account(account: *mut crate::bridge_structs::PurpleAccount) -> Self {
        Self {
            account,
            tx_ptr: std::ptr::null_mut(),
            qrcode: std::ptr::null_mut(),
            uuid: std::ptr::null_mut(),
            debug: -1,
            error: -1,
            connected: 0,
            padding: 0,
            timestamp: 0,
            flags: PurpleMessageFlags(0),
            who: std::ptr::null_mut(),
            name: std::ptr::null_mut(),
            phone_number: std::ptr::null_mut(),
            group: std::ptr::null_mut(),
            body: std::ptr::null_mut(),
            blob: std::ptr::null_mut(),
            size: 0,
            groups: std::ptr::null_mut(),
            roomlist: std::ptr::null_mut(),
            xfer: std::ptr::null_mut(),
        }
    }
}

extern "C" {
    // this is implemented by bridge.c
    fn presage_append_message(message: *const crate::bridge_structs::Message);

    // this is implemented by libpurple's ft.c
    // TODO: automatically generate declaration from ft.h
    fn purple_xfer_get_local_filename(xfer: *mut crate::bridge_structs::PurpleXfer) -> *const std::os::raw::c_char;
}

// wrapper around unsafe presage_append_message
pub fn append_message(message: *const crate::bridge_structs::Message) {
    unsafe {
        presage_append_message(message);
    }
}

// convenience function for calling purple_error on the main thread
pub fn purple_error(
    account: *mut crate::bridge_structs::PurpleAccount,
    level: crate::bridge_structs::PurpleConnectionError,
    msg: String,
) {
    let mut message = crate::bridge_structs::Message::from_account(account);
    message.error = level;
    message.body = std::ffi::CString::new(msg).unwrap().into_raw();
    crate::bridge::append_message(&message);
}

// convenience function for calling purple_debug on the main thread
pub fn purple_debug(
    account: *mut crate::bridge_structs::PurpleAccount,
    level: crate::bridge_structs::PurpleDebugLevel,
    msg: String,
) {
    let mut message = crate::bridge_structs::Message::from_account(account);
    message.debug = level;
    message.body = std::ffi::CString::new(msg).unwrap().into_raw();
    crate::bridge::append_message(&message);
}

// wrapper around unsafe purple_xfer_get_local_filename
pub fn xfer_get_local_filename(xfer: *mut crate::bridge_structs::PurpleXfer) -> String {
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

#[no_mangle]
pub extern "C" fn presage_rust_free_string(c_str: *mut std::os::raw::c_char) {
    if !c_str.is_null() {
        unsafe {
            drop(Box::from_raw(c_str));
        }
    }
}

// TODO: types should be aligned with Presage::blob and Presage::blobsize respectively
#[no_mangle]
pub extern "C" fn presage_rust_free_buffer(
    c_buf: *mut std::os::raw::c_uchar,
    len: std::os::raw::c_ulonglong, // this should be the C equivalent of usize
) {
    if !c_buf.is_null() {
        unsafe {
            drop(Box::from_raw(std::slice::from_raw_parts_mut(c_buf, len as usize)));
        };
    }
}

#[no_mangle]
pub extern "C" fn presage_rust_strfreev(
    c_arr_of_str: *mut *mut std::os::raw::c_char,
    len: std::os::raw::c_ulonglong, // this should be the C equivalent of usize
) {
    if !c_arr_of_str.is_null() {
        unsafe {
            let slice = std::slice::from_raw_parts_mut(c_arr_of_str, len as usize);
            for c_str in &mut *slice {
                presage_rust_free_string(*c_str);
            }
            drop(Box::from_raw(slice));
        };
    }
}

/*
 * Around the core's main function.
 *
 * This blocks until the rust main function finishes.
 */
#[no_mangle]
pub unsafe extern "C" fn presage_rust_main(
    rt: *mut tokio::runtime::Runtime,
    account: *mut crate::bridge_structs::PurpleAccount,
    c_store_path: *const std::os::raw::c_char,
) {
    let store_path = std::ffi::CStr::from_ptr(c_store_path).to_str().unwrap().to_owned();

    // create a channel for asynchronous communication of commands c → rust
    let (tx, rx) = tokio::sync::mpsc::channel(32);
    let tx_ptr = Box::into_raw(Box::new(tx));
    let mut message = crate::bridge_structs::Message::from_account(account);
    message.tx_ptr = tx_ptr as crate::bridge_structs::RustChannelPtr;
    append_message(&message); // let front-end know how to reach us

    // now execute the actual program
    let runtime = rt.as_ref().unwrap();
    runtime.block_on(crate::core::main(store_path, None, rx, account));
    purple_debug(account, crate::bridge_structs::PURPLE_DEBUG_INFO, String::from("rust runtime finishes now…\n"));
}
