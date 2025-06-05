A libpurple/Pidgin protocol plugin for Signal (formerly textsecure) using [presage](https://github.com/whisperfish/presage).

Contains code from [flare](https://gitlab.com/schmiddi-on-mobile/flare) by [Schmiddi](https://github.com/Schmiddiii).

## Download

See the Releases section on github.

## Set-up

1. Create a new Pidgin account. Enter your Signal account UUID as username. In case you do not know your UUID, just enter anything. The plug-in will tell you what to use.
2. Enable the connection. A window with the QR-code should pop-up. Scan it with your master device. Wait for the window to close.

Note: bitlbee users will receive the login QR-code in form of a URI from a system contact "Logon QR Code". You may need to allow unsolicited messages from unknown contacts in your client. The URI can be converted using any tool like a local [`qrencode`](https://www.shellhacks.com/qr-code-generator-windows-linux-macos/) or [online services](https://www.the-qrcode-generator.com/) (use at your own risk).

## Configuration

* `startup-delay-seconds` int  
  Tells the plug-in to wait the specified amount of seconds (default: 1) between spawning the native thread for the rust runtime and actually starting the rust runtime. This magically alleviates database locking issues.

## Features

### Present

* Can link as secondary device via QR-Code.
* Receives a simple text message from a contact or a group.
* Displays quotes, reactions and incoming calls.
* Receives attachments. Special handling for long text messages.
* Can send a simple text message. 
* Can send an attachment.
* Will add buddies to contact list unconditionally.
* Uses special handling of login procedure for bitlbee.
* Can reply to a specific message via "@searchstring:".
* Some very basic support for Spectrum.

### Missing

#### To Be Done Soon™

* Forward all errors to front-end properly.

#### On Hold

* Fetch contact names from the main device.
* Mark messages as "read". This is currently not implemented in back-end, see [#141](https://github.com/whisperfish/presage/issues/141). At time of writing, notifications on main device are deleted after answering via linked device. So that is working alright.
* A group chat is only added to the buddy list when receiving a message. There seems to be no way to fetch the list of groups from the main device, see [#303](https://github.com/whisperfish/presage/issues/303).
* The maximum allowed length of a text-message is unknown.

#### "Contributions Welcome"

* Configuration option whether to add contacts to buddy list or not
* Use the hostname (or a user-defined string) as a device name
* Reasonable generation of C headers and rust constants
* Receive stickers, mentions, styles, contact,…
* Display typing notifications
* Display receipts (not important)
* Support for alternative host applications (Bitlbee)
* Support for adding contact via phone number

These lists are not exhaustive.

### Known Issues

* Contacts are fetched from the main device only once after linking.
* Information about contact names arrive after the first messages.
* Own name is not transferred to the buddy list and therefore not resolved in group chats.
* Sometimes, the database cannot be opened due to locking issues.
* Spectrum support is very flaky. Crashes, infinite loops and silent disconnects may happen. Please keep an eye on your system and check the logs frequently. Issue reports are welcome.

## Building

### Linux

#### Install Dependencies

If your distribution is rolling or very recent, the rust compiler might be recent enough. If not, install rust according to [the rustup instructions](https://www.rust-lang.org/tools/install).

##### Ubuntu and Debian 

    sudo apt install libpurple-dev libqrencode-dev protobuf-compiler

##### Alpine

    doas apk add rust pidgin-dev libqrencode-dev protoc

#### Build

    git clone --recurse-submodules https://github.com/hoehermann/purple-presage purple-presage
    cmake -S purple-presage -B build
    cmake --build build
    sudo cmake --install build

### Windows

purple-presage is known to compile with MSVC 19.30 and rust 1.75. You need the version of rust mentioned in [libsignal-service-rs](https://github.com/whisperfish/libsignal-service-rs/tree/main#note-on-supported-rust-versions). A newer version will probably work, too. Using the "x86 Native Tools Command Prompt for VS 2022" is recommended.

#### Dependencies

Install dependencies via vcpkg:

    vcpkg.exe install libqrencode:x86-windows-static

protoc needs to be in your PATH. You can install it with any method you like, including vcpkg:

    vcpkg.exe install protobuf

#### Build

Same as Linux build instructions, but may need to modify the configuration:

1. Generate MSBuild project:

        cmake -DCMAKE_BUILD_TYPE=Release -DCMAKE_GENERATOR_PLATFORM=WIN32 -DCMAKE_TOOLCHAIN_FILE="…/vcpkg/scripts/buildsystems/vcpkg.cmake" -DVCPKG_TARGET_TRIPLET=x86-windows-static -DRust_CARGO_TARGET="i686-pc-windows-msvc" -S purple-presage -B build

    If necessary, the rust tool-chain version can be specified via `-DRust_TOOLCHAIN="1.75-i686-pc-windows-msvc"`.

2. Build, Install and Run:

        cmake --build build
        cmake --install build --config Release
        cmake --build build --target run

When using the "Debug" configuration, the linker complains about mismatching configurations. The implications of this are unknown.

#### Notes

On Windows, purple-presage must be built with MSVC. gcc (via MinGW or MSYS2) has a number of issues such as [incompatibility with recent rustc versions](https://github.com/rust-lang/rust/issues/112368) and not shipping libbcrypt by default.

Needs a whooping 6 GB of disk space during build! 😳
