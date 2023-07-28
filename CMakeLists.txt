cmake_minimum_required(VERSION 3.8)

project("purple-presage")

set(Purple_DIR "${CMAKE_CURRENT_SOURCE_DIR}/submodules/purple-cmake" CACHE PATH "Where to find purple-config.cmake")
find_package(Purple REQUIRED)
message(STATUS "PURPLE_INCLUDE_DIRS: ${PURPLE_INCLUDE_DIRS}")
message(STATUS "PURPLE_LIBRARY_DIRS: ${PURPLE_LIBRARY_DIRS}")
message(STATUS "PURPLE_LIBRARIES: ${PURPLE_LIBRARIES}")
message(STATUS "PURPLE_PLUGIN_DIR: ${PURPLE_PLUGIN_DIR}")
message(STATUS "PURPLE_DATA_DIR: ${PURPLE_DATA_DIR}")

find_path(QRENCODE_INCLUDE_DIR NAMES qrencode.h)
find_library(QRENCODE_LIBRARIES qrencode)

include(FetchContent)
FetchContent_Declare( Corrosion GIT_REPOSITORY https://github.com/corrosion-rs/corrosion.git)
FetchContent_MakeAvailable(Corrosion)
set(Rust_PROFILE "release" CACHE STRING "dev or release")
corrosion_import_crate(MANIFEST_PATH src/rust/Cargo.toml PROFILE ${Rust_PROFILE})

set(TARGET_NAME purple-presage)
add_library(${TARGET_NAME} SHARED 
    "src/c/init.c"
    "src/c/bridge.c"
    "src/c/connection.c"
    "src/c/qrcode.c"
    "src/c/text.c"
    "src/c/direct.c"
    "src/c/contact.c"
    "src/c/presage.h"
)
target_link_libraries(${TARGET_NAME} PRIVATE purple_presage_backend)
target_include_directories(${TARGET_NAME} PRIVATE ${QRENCODE_INCLUDE_DIR})
target_link_libraries(${TARGET_NAME} PRIVATE ${QRENCODE_LIBRARIES})

if (WIN32)
    target_link_libraries(${TARGET_NAME} PRIVATE ws2_32.lib Crypt32.lib Ncrypt.lib Secur32.lib Userenv.lib NtDll.lib)
else()
    target_link_libraries(purple_presage_backend INTERFACE m ${TARGET_NAME})
    # additional libraries obtained via cargo rustc -- --print native-static-libs :
    # gcc_s util rt pthread m dl c
endif()

file(READ "${CMAKE_SOURCE_DIR}/VERSION" PLUGIN_VERSION)
target_compile_definitions(${TARGET_NAME} PRIVATE PLUGIN_VERSION=${PLUGIN_VERSION})
target_include_directories(${TARGET_NAME} PRIVATE ${PURPLE_INCLUDE_DIRS})
target_link_libraries(${TARGET_NAME} PRIVATE ${PURPLE_LIBRARIES})
set_target_properties(${TARGET_NAME} PROPERTIES PREFIX "lib")

if (WIN32)
    install(TARGETS ${TARGET_NAME} RUNTIME DESTINATION "${PURPLE_PLUGIN_DIR}")
else()
    install(TARGETS ${TARGET_NAME} DESTINATION "${PURPLE_PLUGIN_DIR}")
endif()

install(DIRECTORY "pixmaps" DESTINATION "${PURPLE_DATA_DIR}" FILES_MATCHING PATTERN "*.png")
