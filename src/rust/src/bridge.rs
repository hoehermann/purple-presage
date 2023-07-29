#[repr(C)]
pub struct Presage {
    pub account: *const std::os::raw::c_void,
    pub tx_ptr: *mut std::os::raw::c_void,
    pub qrcode: *const std::os::raw::c_char,
    pub uuid: *const std::os::raw::c_char,

    // TODO: find out how to use stdint on Windows
    pub timestamp: std::os::raw::c_ulonglong, //stdint::uint64_t,
    pub sent: std::os::raw::c_ulonglong,      //stdint::uint64_t,
    pub who: *const std::os::raw::c_char,
    pub group: *const std::os::raw::c_char,
    pub body: *const std::os::raw::c_char,
}

impl Presage {
    pub fn from_account(account: *const std::os::raw::c_void) -> Self {
        Self {
            account: account,
            tx_ptr: std::ptr::null_mut(),
            qrcode: std::ptr::null(),
            uuid: std::ptr::null(),
            timestamp: 0,
            sent: 0,
            who: std::ptr::null(),
            group: std::ptr::null(),
            body: std::ptr::null(),
        }
    }
}

extern "C" {
    // this is implemented by bridge.c
    fn presage_append_message(message: *const Presage);
}

// wrapper around unsafe presage_append_message
pub fn append_message(message: *const Presage) {
    unsafe {
        presage_append_message(message);
    }
}

/*
 * This library has no main function to annotate with `#[tokio::main]`, but needs a run-time. 
 * This function creates a tokio runtime and boxes it so the runtime can live in the front-end.
 * 
 * https://stackoverflow.com/questions/66196972/ and https://stackoverflow.com/questions/64658556/ are helpful.
 */
#[no_mangle]
pub extern "C" fn presage_rust_init() -> *mut tokio::runtime::Runtime {
    let runtime = tokio::runtime::Builder::new_multi_thread().thread_name("presage Tokio").enable_io().enable_time().build().unwrap();
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
pub extern "C" fn presage_rust_free(c_str: *mut std::os::raw::c_char) {
    if c_str == std::ptr::null_mut() {
        return;
    }
    unsafe {
        drop(Box::from_raw(c_str));
    }
}

/*
 * Around the core's main function.
 * 
 * According to https://docs.rs/tokio/latest/tokio/task/struct.LocalSet.html, 
 * the top call must be blocking. So this blocks until the main function finishes.
 */
#[no_mangle]
pub unsafe extern "C" fn presage_rust_main(
    rt: *mut tokio::runtime::Runtime,
    account: *const std::os::raw::c_void,
    c_store_path: *const std::os::raw::c_char,
) {
    let store_path = std::ffi::CStr::from_ptr(c_store_path).to_str().unwrap().to_owned();
    
    // create a channel for asynchronous communication of commands c → rust
    let (tx, rx) = tokio::sync::mpsc::channel(32);
    let tx_ptr = Box::into_raw(Box::new(tx));
    let mut message = Presage::from_account(account);
    message.tx_ptr = tx_ptr as *mut std::os::raw::c_void;
    append_message(&message); // let front-end know how to reach us
    
    // now execute the actual program
    let runtime = rt.as_ref().unwrap();
    runtime.block_on(async {
        let local = tokio::task::LocalSet::new();
        local.run_until(crate::core::main(store_path, None, rx, account)).await;
    });
    println!("rust: main finished.");
}
