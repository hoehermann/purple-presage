An attempt to use [presage](https://github.com/whisperfish/presage) in libpurple.

For now, these are random notes taken during erratic development:

needs a whooping 6 GB of disk space! :o

MinGW

mingw-get install pthreads-dev
mingw-get install msys-libcrypt
mingw-get install msys-libopenssl-dev

rustup-init --default-toolchain=1.69 --default-host=i686-pc-windows-gnu

https://github.com/rg3/libbcrypt/archive/refs/heads/master.zip
"c:\\Users\\Hermann\\source\\repos\\purple-presage\\src\\rust\\target\\debug\\deps"
C:\Users\Hermann\.rustup\toolchains\stable-i686-pc-windows-gnu\lib\rustlib\i686-pc-windows-gnu\lib

https://github.com/rust-lang/rust/issues/112368

https://stackoverflow.com/questions/28500658/mingw-undefined-reference-to-mingw-glob-when-using-ws2-32-library


libqrencode:x86-windows-static

cmake -DCMAKE_BUILD_TYPE=Debug -GNinja -DCMAKE_PREFIX_PATH=C:/Users/Hermann/source/vcpkg/installed/x86-windows -DRust_TOOLCHAIN="1.69-i686-pc-windows-msvc" -DRust_CARGO_TARGET="i686-pc-windows-msvc" ..
