cmake_minimum_required(VERSION 3.19) # for string(JSON …)
# 3.18 needed for win32 part of submodules/purple-cmake

project("purple-presage")

message(STATUS "C Compiler Path: ${CMAKE_C_COMPILER}")
if(WIN32)
    if (MSVC)
        # in MSVC, target architecture is set by environment
    else()
        # not MSVC, assuming GCC
        add_compile_options(-m32)
        add_link_options(-m32)
        # sadly, the above have no effect on CMAKE_SIZEOF_VOID_P, so I force it 
        # also see https://stackoverflow.com/questions/43379311/why-does-project-affect-cmakes-opinion-on-cmake-sizeof-void-p
        set(CMAKE_SIZEOF_VOID_P 4)
    endif()
endif()
# try to limit to Windows 7 as per user request
add_compile_definitions(_WIN32_WINNT=0x0601 WINVER=0x0601)

set(Purple_DIR "${CMAKE_CURRENT_SOURCE_DIR}/submodules/purple-cmake" CACHE PATH "Where to find purple-config.cmake")
find_package(Purple REQUIRED)
message(STATUS "PURPLE_INCLUDE_DIRS: ${PURPLE_INCLUDE_DIRS}")
message(STATUS "PURPLE_LIBRARY_DIRS: ${PURPLE_LIBRARY_DIRS}")
message(STATUS "PURPLE_LIBRARIES: ${PURPLE_LIBRARIES}")
message(STATUS "PURPLE_PLUGIN_DIR: ${PURPLE_PLUGIN_DIR}")
message(STATUS "PURPLE_DATA_DIR: ${PURPLE_DATA_DIR}")
message(STATUS "PURPLE_VERSION: ${PURPLE_VERSION}")

find_path(QRENCODE_INCLUDE_DIR NAMES qrencode.h)
find_library(QRENCODE_LIBRARIES qrencode)

find_package(OpenSSL REQUIRED)
cmake_path(GET OPENSSL_INCLUDE_DIR PARENT_PATH OPENSSL_DIR)

find_package(PkgConfig QUIET)
if (${PKG_CONFIG_FOUND})
    pkg_check_modules(PIXBUF gdk-pixbuf-2.0)
endif()

include(FetchContent)
FetchContent_Declare(Corrosion GIT_REPOSITORY https://github.com/corrosion-rs/corrosion.git)
FetchContent_MakeAvailable(Corrosion)
set(Rust_PROFILE "release" CACHE STRING "Rust profile")
set_property(CACHE Rust_PROFILE PROPERTY STRINGS "dev" "release")
corrosion_import_crate(MANIFEST_PATH ${CMAKE_CURRENT_SOURCE_DIR}/src/rust/Cargo.toml PROFILE ${Rust_PROFILE} FLAGS ${Rust_FLAGS})
corrosion_set_env_vars(purple_presage_backend OPENSSL_DIR=${OPENSSL_DIR})
corrosion_add_target_rustflags(purple_presage_backend "-C target-feature=+crt-static")
# https://stackoverflow.com/questions/31770604/how-to-generate-statically-linked-executables
# maybe also look at https://github.com/chrisdenton/static_vcruntime

# obtain version string from back-end metadata
execute_process(COMMAND ${Rust_CARGO_CACHED} metadata --format-version 1 --no-deps --manifest-path ${CMAKE_CURRENT_SOURCE_DIR}/src/rust/Cargo.toml OUTPUT_VARIABLE BACKEND_METADATA)
string(JSON BACKEND_VERSION GET ${BACKEND_METADATA} packages 0 version)
string(JSON BACKEND_DEPENDENCIES GET ${BACKEND_METADATA} packages 0 dependencies)
execute_process(COMMAND git -C ${CMAKE_CURRENT_SOURCE_DIR} log -1 --date=format:%Y%m%d --format=%ad OUTPUT_VARIABLE COMMIT_DATE OUTPUT_STRIP_TRAILING_WHITESPACE)
execute_process(COMMAND git -C ${CMAKE_CURRENT_SOURCE_DIR} rev-list --count HEAD OUTPUT_VARIABLE REVISION_COUNT OUTPUT_STRIP_TRAILING_WHITESPACE)

string(JSON DEPENDENCY_COUNT LENGTH "${BACKEND_DEPENDENCIES}")
math(EXPR DEPENDENCY_COUNT "${DEPENDENCY_COUNT}-1")
foreach(i RANGE ${DEPENDENCY_COUNT})
  string(JSON BACKEND_DEPENDENCY GET "${BACKEND_DEPENDENCIES}" ${i})
  string(JSON BACKEND_DEPENDENCY_NAME GET "${BACKEND_DEPENDENCY}" "name")
  if("${BACKEND_DEPENDENCY_NAME}" STREQUAL "presage")
    string(JSON PRESAGE_SOURCE GET "${BACKEND_DEPENDENCY}" "source")
    string(REGEX MATCH "[^=]+$" PRESAGE_COMMIT "${PRESAGE_SOURCE}")
  endif()
endforeach()
set(PLUGIN_VERSION "${BACKEND_VERSION}_${COMMIT_DATE}_${REVISION_COUNT}_${PRESAGE_COMMIT}" CACHE STRING "Plug-In Version")
message(STATUS "PLUGIN_VERSION: ${PLUGIN_VERSION}")

set(TARGET_NAME "presage")
add_library(${TARGET_NAME} SHARED
    "src/c/bridge.c"
    "src/c/connection.c"
    "src/c/qrcode.c"
    "src/c/receive_text.c"
    "src/c/send_text.c"
    "src/c/blist.c"
    "src/c/status.c"
    "src/c/groups.c"
    "src/c/receive_attachment.c"
    "src/c/send_file.c"
    "src/c/profile.c"
    "src/c/options.c"
    "src/c/presage.h"
    "src/c/attachment_common.c"
    "src/c/init.c"
    "src/c/pixbuf.c"
)
target_link_libraries(${TARGET_NAME} PRIVATE purple_presage_backend)
target_include_directories(${TARGET_NAME} PRIVATE ${QRENCODE_INCLUDE_DIR})
target_link_libraries(${TARGET_NAME} PRIVATE ${QRENCODE_LIBRARIES})
if (MSVC)
    add_dependencies(${TARGET_NAME} libpurple_lib)
    target_link_libraries(${TARGET_NAME} PRIVATE OpenSSL::Crypto ws2_32.lib Crypt32.lib Ncrypt.lib Secur32.lib Userenv.lib NtDll.lib RuntimeObject.lib)
    set_property(TARGET ${TARGET_NAME} PROPERTY MSVC_RUNTIME_LIBRARY "MultiThreaded") # link vcruntime140 release version statically, see https://cmake.org/cmake/help/latest/prop_tgt/MSVC_RUNTIME_LIBRARY.html
else()
    target_link_libraries(purple_presage_backend INTERFACE crypto m ${TARGET_NAME})
    # additional libraries obtained via cargo rustc -- --print native-static-libs :
    # gcc_s util rt pthread m dl c
endif()
set_property(TARGET ${TARGET_NAME} PROPERTY COMPILE_WARNING_AS_ERROR ON) # needs cmake 3.24, but is not critical

target_compile_definitions(${TARGET_NAME} PRIVATE PLUGIN_VERSION=${PLUGIN_VERSION})
target_include_directories(${TARGET_NAME} PRIVATE ${PURPLE_INCLUDE_DIRS} ${PIXBUF_INCLUDE_DIRS})
target_link_libraries(${TARGET_NAME} PRIVATE ${PURPLE_LIBRARIES} ${PIXBUF_LIBRARIES})
set_target_properties(${TARGET_NAME} PROPERTIES PREFIX "lib")

if (WIN32)
    install(TARGETS ${TARGET_NAME} RUNTIME DESTINATION "${PURPLE_PLUGIN_DIR}")
else()
    install(TARGETS ${TARGET_NAME} DESTINATION "${PURPLE_PLUGIN_DIR}")
endif()

install(DIRECTORY "assets/pixmaps" DESTINATION "${PURPLE_DATA_DIR}" FILES_MATCHING PATTERN "*.png")

include(cpack.cmake)
