A proof of concept using [presage](https://github.com/whisperfish/presage) in libpurple.

### Download

* [Latest Build for Windows](https://nightly.link/hoehermann/purple-presage/workflows/build/master/libpresage.dll.zip)

### Set-up

1. Create a new Pidgin account. Enter your Signal account UUID as username. In case you do not know your UUID, just enter anything. The plug-in will tell you what to use.
2. Enable the connection. A window with the QR-code should pop-up. Scan it with your master device. Wait for the window to close.

### Features

#### Present

* Can link against a master device via QR-Code.
* Receives a simple text message from a contact or a group.
* Can reply with a simple text message.
* Can add contacts to buddy list manually.
* Will add buddies to contact list unconditionally.
* Receives attachments (only pictures tested for now).

#### Missing

Everything else, most notably:

* Error handling
* Reasonable contact and group management (list of participants,…)
* Stickers, mentions, replies, quotes, styles,…
* Mark messages as "read"
* Reply to a specific message
* Display typing… notifications
* Display receipts
* Support for alternative UIs
* Support for phone numbers

This list is not exhaustive.

### Known Issues

* Sometimes, when sending a message to initiate a conversation, it never reaches destination. Since there is almost no error handling, the user cannot to know for sure.

### Building

#### Linux

##### Dependencies

* `libpurple-dev`
* `libqrencode-dev`
* `protobuf` (or any other package which provides the `protoc` compiler)

##### Build

    git clone --recurse-submodules https://github.com/hoehermann/purple-presage
    mkdir purple-presage/build
    cd purple-presage/build
    cmake ..
    cmake --build .
    sudo cmake --install .

#### Windows

purple-presage is known to compile with MSVC 19.30 and rust 1.75. You need the version of rust mentioned in [libsignal-service-rs](https://github.com/whisperfish/libsignal-service-rs/tree/main#note-on-supported-rust-versions). A newer version will probably work, too. Using the "x86 Native Tools Command Prompt for VS 2022" is recommended.

##### Dependencies

Install dependencies via vcpkg:

    vcpkg.exe install libqrencode:x86-windows-static

protoc needs to be in your PATH. You can install it with any method you like, including vcpkg:

    vcpkg.exe install protobuf

##### Build

Same as Linux build instructions, but may need to modify the configuration:

1. Generate project:

        cmake -DCMAKE_BUILD_TYPE=Debug -DCMAKE_TOOLCHAIN_FILE="…/vcpkg/scripts/buildsystems/vcpkg.cmake" -DVCPKG_TARGET_TRIPLET=x86-windows-static -DRust_CARGO_TARGET="i686-pc-windows-msvc" ..

    If necessary, the rust toolchain version can be specified via `-DRust_TOOLCHAIN="1.75-i686-pc-windows-msvc"`.

2. Build, Install and Run:

        cmake --build .
        cmake --install .
        cmake --build . --target run

##### Notes

purple-presage must be built with MSVC. MinGW's GCC encountered a number of issues such as [incompatibility with rustc versions newer than 1.69](https://github.com/rust-lang/rust/issues/112368) and not shipping libbcrypt by default.

Needs a whooping 6 GB of disk space during build! :o
