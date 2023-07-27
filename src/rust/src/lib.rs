//#![no_std]
#![no_main]

use futures::StreamExt;
use futures::{channel::oneshot, future};
use presage::Store;
use presage::{prelude::SignalServers, Manager};
use presage_store_sled::{MigrationConflictStrategy, SledStore};

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
    fn presage_append_message(input: *const Presage);
}

// https://stackoverflow.com/questions/66196972/how-to-pass-a-reference-pointer-to-a-rust-struct-to-a-c-ffi-interface
#[no_mangle]
pub extern "C" fn presage_rust_init() -> *mut tokio::runtime::Runtime {
    // https://stackoverflow.com/questions/64658556/
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .thread_name("presage Tokio")
        .enable_io()
        .enable_time()
        .build()
        .unwrap();
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

fn print_message<C: Store>(
    manager: &Manager<C, presage::Registered>,
    content: &presage::prelude::Content,
    account: *const std::os::raw::c_void,
) {
    let Ok(thread) = presage::Thread::try_from(content) else {
        println!("rust: failed to derive thread from content");
        return;
    };
    let mut message = Presage::from_account(account);

    let format_data_message =
        |thread: &presage::Thread, data_message: &presage::prelude::content::DataMessage| {
            match data_message {
                presage::prelude::content::DataMessage {
                    quote:
                        Some(presage::prelude::proto::data_message::Quote {
                            text: Some(quoted_text),
                            ..
                        }),
                    body: Some(body),
                    ..
                } => Some(format!("Answer to message \"{quoted_text}\": {body}")),
                presage::prelude::content::DataMessage {
                    reaction:
                        Some(presage::prelude::proto::data_message::Reaction {
                            target_sent_timestamp: Some(timestamp),
                            emoji: Some(emoji),
                            ..
                        }),
                    ..
                } => {
                    let Ok(Some(message)) = manager.message(thread, *timestamp) else {
                println!("rust: no message in {thread} sent at {timestamp}");
                return None;
            };

                    let presage::prelude::content::ContentBody::DataMessage(presage::prelude::DataMessage { body: Some(body), .. }) = message.body else {
                println!("rust: message reacted to has no body");
                return None;
            };

                    Some(format!("Reacted with {emoji} to message: \"{body}\""))
                }
                presage::prelude::content::DataMessage {
                    body: Some(body), ..
                } => Some(body.to_string()),
                _ => Some("Empty data message".to_string()),
            }
        };

    /*
    let format_contact = |uuid| {
        manager
            .contact_by_id(uuid)
            .ok()
            .flatten()
            .filter(|c| !c.name.is_empty())
            .map(|c| format!("{}: {}", c.name, uuid))
            .unwrap_or_else(|| uuid.to_string())
    };
    let group_get_title = |key| {
        manager
            .group(key)
            .ok()
            .flatten()
            .map(|g| g.title)
            .unwrap_or_else(|| "<missing group>".to_string())
    };
    */

    enum Msg<'a> {
        Received(&'a presage::Thread, String),
        Sent(&'a presage::Thread, String),
    }

    if let Some(msg) = match &content.body {
        presage::prelude::content::ContentBody::NullMessage(_) => Some(Msg::Received(
            &thread,
            "Null message (for example deleted)".to_string(),
        )),
        presage::prelude::content::ContentBody::DataMessage(data_message) => {
            format_data_message(&thread, data_message).map(|body| Msg::Received(&thread, body))
        }
        presage::prelude::content::ContentBody::SynchronizeMessage(
            presage::prelude::SyncMessage {
                sent:
                    Some(presage::prelude::proto::sync_message::Sent {
                        message: Some(data_message),
                        ..
                    }),
                ..
            },
        ) => format_data_message(&thread, data_message).map(|body| Msg::Sent(&thread, body)),
        presage::prelude::content::ContentBody::CallMessage(_) => {
            Some(Msg::Received(&thread, "is calling!".into()))
        }
        // TODO: forward this as typing message
        //presage::prelude::content::ContentBody::TypingMessage(_) => Some(Msg::Received(&thread, "is typing...".into())),
        c => {
            println!("rust: unsupported message {c:?}");
            None
        }
    } {
        let (who, group, body, sent) = match msg {
            Msg::Received(presage::Thread::Contact(sender), body) => {
                (sender.to_string(), String::from(""), body, false)
            }
            Msg::Sent(presage::Thread::Contact(recipient), body) => {
                (recipient.to_string(), String::from(""), body, true)
            }
            Msg::Received(presage::Thread::Group(key), body) => {
                let group = hex::encode(key);
                (content.metadata.sender.uuid.to_string(), group, body, false)
            }
            Msg::Sent(presage::Thread::Group(key), body) => {
                let group = hex::encode(key);
                (String::from(""), group, body, true)
            }
        };

        println!("{who} in {group} wrote {body}");
        message.timestamp = content.metadata.timestamp;
        message.sent = if sent { 1 } else { 0 };
        if who != "" {
            message.who = std::ffi::CString::new(who).unwrap().into_raw();
        }
        if group != "" {
            message.group = std::ffi::CString::new(group).unwrap().into_raw();
        }
        message.body = std::ffi::CString::new(body).unwrap().into_raw();
        unsafe {
            presage_append_message(&message);
        }
    }
}

async fn process_incoming_message<C: Store>(
    manager: &mut Manager<C, presage::Registered>,
    content: &presage::prelude::Content,
    account: *const std::os::raw::c_void,
) {
    print_message(manager, content, account);

    /*
    let sender = content.metadata.sender.uuid;
    if let ContentBody::DataMessage(DataMessage { attachments, .. }) = &content.body {
        for attachment_pointer in attachments {
            let Ok(attachment_data) = manager.get_attachment(attachment_pointer).await else {
                log::warn!("failed to fetch attachment");
                continue;
            };

            let extensions = mime_guess::get_mime_extensions_str(
                attachment_pointer
                    .content_type
                    .as_deref()
                    .unwrap_or("application/octet-stream"),
            );
            let extension = extensions.and_then(|e| e.first()).unwrap_or(&"bin");
            let filename = attachment_pointer
                .file_name
                .clone()
                .unwrap_or_else(|| Local::now().format("%Y-%m-%d-%H-%M-%s").to_string());
            let file_path = attachments_tmp_dir.join(format!("presage-{filename}.{extension}",));
            match fs::write(&file_path, &attachment_data).await {
                Ok(_) => info!("saved attachment from {sender} to {}", file_path.display()),
                Err(error) => error!(
                    "failed to write attachment from {sender} to {}: {error}",
                    file_path.display()
                ),
            }
        }
    }
    */
}

async fn receive<C: Store>(
    manager: &mut Manager<C, presage::Registered>,
    account: *const std::os::raw::c_void,
) {
    let messages = manager.receive_messages().await.unwrap(); // TODO: add error handling instead of unwrap

    futures::pin_mut!(messages);

    while let Some(content) = messages.next().await {
        process_incoming_message(manager, &content, account).await;
    }
}

// from main
pub enum Cmd {
    LinkDevice {
        servers: SignalServers,
        device_name: String,
    },
    Exit,
    Whoami,
    Receive,
    Send {
        uuid: presage::prelude::Uuid,
        message: String,
    },
}

async fn send<C: Store + 'static>(
    msg: &str,
    uuid: &presage::prelude::Uuid,
    manager: &mut Manager<C, presage::Registered>,
) {
    let timestamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .expect("Time went backwards")
        .as_millis() as u64;

    let message = presage::prelude::ContentBody::DataMessage(presage::prelude::DataMessage {
        body: Some(msg.to_string()),
        timestamp: Some(timestamp),
        ..Default::default()
    });

    manager
        .send_message(*uuid, message, timestamp)
        .await
        .unwrap();
}

async fn run<C: Store + 'static>(
    subcommand: Cmd,
    config_store: C,
    manager: Option<Manager<C, presage::Registered>>,
    account: *const std::os::raw::c_void,
) -> Result<Manager<C, presage::Registered>, presage::Error<<C>::Error>> {
    match subcommand {
        Cmd::LinkDevice {
            servers,
            device_name,
        } => {
            let (provisioning_link_tx, provisioning_link_rx) = oneshot::channel();
            let join_handle = future::join(
                Manager::link_secondary_device(
                    config_store,
                    servers,
                    device_name.clone(),
                    provisioning_link_tx,
                ),
                async move {
                    match provisioning_link_rx.await {
                        Ok(url) => {
                            println!("rust: qr code ok.");
                            println!("rust: now calling presage_append_message…");
                            let mut message = Presage::from_account(account);
                            message.qrcode =
                                std::ffi::CString::new(url.to_string()).unwrap().into_raw();
                            unsafe {
                                presage_append_message(&message);
                            }
                        }
                        Err(e) => println!("Error linking device: {e}"),
                    }
                },
            )
            .await;

            let mut message = Presage::from_account(account);
            let qrcode_done = String::from("");
            message.qrcode = std::ffi::CString::new(qrcode_done).unwrap().into_raw();
            unsafe {
                presage_append_message(&message);
            }
            let (manager, _) = join_handle;
            manager
        }

        Cmd::Whoami => {
            let mut uuid = String::from("");
            let manager = manager.unwrap_or(Manager::load_registered(config_store).await?);
            let whoami = manager.whoami().await?;
            uuid = whoami.uuid.to_string();
            // TODO: find out if this one is showing ServiceError(Unauthorized)
            let mut message = Presage::from_account(account);
            message.uuid = std::ffi::CString::new(uuid.to_string()).unwrap().into_raw();
            unsafe {
                presage_append_message(&message);
            }
            Ok(manager)
        }

        Cmd::Receive => {
            let manager = manager.unwrap_or(Manager::load_registered(config_store).await?);
            let mut receiving_manager = manager.clone();
            tokio::task::spawn_local(async move {
                receive(&mut receiving_manager, account).await
            });
            Ok(manager)
        }

        Cmd::Send { uuid, message } => {
            let mut manager = manager.unwrap_or(Manager::load_registered(config_store).await?);
            send(&message, &uuid, &mut manager).await;
            Ok(manager)
        }
        
        Cmd::Exit { } => {
            //Err(std::error::Error::from("Exit requested."))
            // TODO: return an error
            Ok(manager.unwrap_or(Manager::load_registered(config_store).await?))
        }
    }
}

async fn mainloop(config_store: SledStore, mut rx: tokio::sync::mpsc::Receiver<Cmd>, account: *const std::os::raw::c_void) {
    let mut manager: Option<Manager<SledStore, presage::Registered>> = None;
    while let Some(cmd) = rx.recv().await {
        match cmd {
            Cmd::Exit => {
                break;
            }
            _ => {
                println!("rust: run begins…");
                // TODO: find out if config_store.clone() is the correct thing to do here
                match run(cmd, config_store.clone(), manager, account).await {
                    Ok(m) => {
                        manager = Some(m);
                    }
                    Err(err) => {
                        manager = None;
                        println!("rust: run Err {err:?}");
                    }
                }
                println!("rust: run finished.");
            }
        }
    }
}

#[no_mangle]
pub unsafe extern "C" fn presage_rust_main(
    rt: *mut tokio::runtime::Runtime,
    account: *const std::os::raw::c_void,
    c_store_path: *const std::os::raw::c_char,
) {
    let store_path: String = std::ffi::CStr::from_ptr(c_store_path)
        .to_str()
        .unwrap()
        .to_owned();
    let (tx, rx) = tokio::sync::mpsc::channel(32);
    let tx_ptr = Box::into_raw(Box::new(tx));
    let mut message = Presage::from_account(account);
    message.tx_ptr = tx_ptr as *mut std::os::raw::c_void;
    unsafe {
        presage_append_message(&message);
    }
    let runtime = rt.as_ref().unwrap();
    runtime.block_on(async {
        let local = tokio::task::LocalSet::new();
        local.run_until(async {
            // from main
            let passphrase: Option<String> = None;
            //println!("rust: opening config database from {store_path}");
            let config_store = SledStore::open_with_passphrase(
                store_path,
                passphrase,
                MigrationConflictStrategy::Raise,
            );
            match config_store {
                Err(err) => {
                    println!("rust: config_store Err {err:?}");
                }
                Ok(config_store) => {
                    println!("rust: config_store OK");
                    mainloop(config_store, rx, account).await;
                }
            }
        }).await;
    });
    println!("rust: main finished.");
}

unsafe fn send_cmd(
    rt: *mut tokio::runtime::Runtime,
    tx: *mut tokio::sync::mpsc::Sender<Cmd>,
    cmd: Cmd,
) {
    let command_tx = tx.as_ref().unwrap();
    let runtime = rt.as_ref().unwrap();
    match runtime.block_on(command_tx.send(cmd)) {
        Ok(()) => {
            //println!("rust: command_tx.send OK");
        }
        Err(err) => {
            println!("rust: command_tx.send {err}");
        }
    }
}

#[no_mangle]
pub unsafe extern "C" fn presage_rust_link(
    rt: *mut tokio::runtime::Runtime,
    tx: *mut tokio::sync::mpsc::Sender<Cmd>,
    c_device_name: *const std::os::raw::c_char,
) {
    let device_name: String = std::ffi::CStr::from_ptr(c_device_name)
        .to_str()
        .unwrap()
        .to_owned();
    println!("rust: presage_rust_link invoked successfully! device_name is {device_name}");
    // from args
    let server: SignalServers = SignalServers::Production;
    //let server: SignalServers = SignalServers::Staging;
    let cmd: Cmd = Cmd::LinkDevice {
        device_name: device_name,
        servers: server,
    };
    send_cmd(rt, tx, cmd);
    println!("rust: presage_rust_link ends now");
}

#[no_mangle]
pub unsafe extern "C" fn presage_rust_stop(
    rt: *mut tokio::runtime::Runtime,
    tx: *mut tokio::sync::mpsc::Sender<Cmd>,
) {
    let cmd: Cmd = Cmd::Exit {};
    send_cmd(rt, tx, cmd);
}

#[no_mangle]
pub unsafe extern "C" fn presage_rust_exit(
    rt: *mut tokio::runtime::Runtime,
    tx: *mut tokio::sync::mpsc::Sender<Cmd>,
) {
    let cmd: Cmd = Cmd::Exit {};
    send_cmd(rt, tx, cmd);
}

#[no_mangle]
pub unsafe extern "C" fn presage_rust_whoami(
    rt: *mut tokio::runtime::Runtime,
    tx: *mut tokio::sync::mpsc::Sender<Cmd>,
) {
    let cmd: Cmd = Cmd::Whoami {};
    send_cmd(rt, tx, cmd);
}

#[no_mangle]
pub unsafe extern "C" fn presage_rust_receive(
    rt: *mut tokio::runtime::Runtime,
    tx: *mut tokio::sync::mpsc::Sender<Cmd>,
) {
    let cmd: Cmd = Cmd::Receive {};
    send_cmd(rt, tx, cmd);
}

#[no_mangle]
pub unsafe extern "C" fn presage_rust_send(
    rt: *mut tokio::runtime::Runtime,
    tx: *mut tokio::sync::mpsc::Sender<Cmd>,
    c_uuid: *const std::os::raw::c_char,
    c_message: *const std::os::raw::c_char,
) {
    let cmd: Cmd = Cmd::Send {
        // TODO: add error handling instead of unwrap()
        uuid: presage::prelude::Uuid::parse_str(std::ffi::CStr::from_ptr(c_uuid)
            .to_str().unwrap()).unwrap(),
        message: std::ffi::CStr::from_ptr(c_message)
            .to_str()
            .unwrap()
            .to_owned(),
    };
    send_cmd(rt, tx, cmd);
}
