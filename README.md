An attempt to use [presage](https://github.com/whisperfish/presage) in libpurple.

For now, these are random notes taken during erratic development:

needs a whooping 6 GB of disk space! :o

### Windows

Presage must be built with MSVC. MinGW's GCC encountered a number of issues such as [incompatibility with rustc versions newer than 1.69](https://github.com/rust-lang/rust/issues/112368) and not shipping libbcrypt by default.

libqrencode:x86-windows-static

cmake -DCMAKE_BUILD_TYPE=Debug -GNinja -DCMAKE_PREFIX_PATH=wherever/vcpkg/installed/x86-windows -DRust_CARGO_TARGET="i686-pc-windows-msvc" ..

-DRust_TOOLCHAIN="1.69-i686-pc-windows-msvc"
