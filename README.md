A proof of concept using [presage](https://github.com/whisperfish/presage) in libpurple.

### Linux

#### Build

    git clone --recurse-submodules https://github.com/hoehermann/purple-presage
    mkdir purple-presage/build
    cd purple-presage/build
    cmake ..
    cmake --build .
    sudo cmake --install .

### Windows

purple-presage is known to compile with MSVC 19.30 and rust 1.71. Using the "x86 Native Tools Command Prompt for VS 2022" is recommended.

#### Dependencies

Install dependencies via vcpkg:

    vcpkg.exe install libqrencode:x86-windows-static

#### Build

Same as Linux build instructions, but may need to modify:

1. Generate project:

        cmake -DCMAKE_BUILD_TYPE=Debug -GNinja -DCMAKE_PREFIX_PATH=wherever/vcpkg/installed/x86-windows -DRust_CARGO_TARGET="i686-pc-windows-msvc" ..

    If necessary, the rust toolchain version can be specified via `-DRust_TOOLCHAIN="1.69-i686-pc-windows-msvc"`.

2. Build, Install and Run:

        cmake --build .
        cmake --install .
        cmake --build . --target run

#### Notes

purple-presage must be built with MSVC. MinGW's GCC encountered a number of issues such as [incompatibility with rustc versions newer than 1.69](https://github.com/rust-lang/rust/issues/112368) and not shipping libbcrypt by default.

Needs a whooping 6 GB of disk space during build! :o
