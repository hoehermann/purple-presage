cmake_minimum_required(VERSION 3.8)

project("purple-presage")

if(WIN32 AND NOT ${CMAKE_SIZEOF_VOID_P} EQUAL 4)
    message(FATAL_ERROR "On Windows, Pidgin 2 is 32 bit only.")
endif()

find_package(PkgConfig QUIET)
if (${PKG_CONFIG_FOUND})
    pkg_check_modules(PURPLE purple)
    pkg_get_variable(PURPLE_PLUGIN_DIR purple plugindir)
    pkg_get_variable(PURPLE_DATA_DIR purple datarootdir)
endif()
message(STATUS "PkgConfig_FOUND is [${PkgConfig_FOUND}] Purple_FOUND is [${Purple_FOUND}]")
if(WIN32 AND (NOT "${PkgConfig_FOUND}" OR NOT "${Purple_FOUND}"))
    message(STATUS "Trying win32 auto-configuration...")
    add_subdirectory(${CMAKE_SOURCE_DIR}/dependencies/win32)
endif()
if(NOT "${Purple_FOUND}")
    if (WIN32)
        message(FATAL_ERROR "Purple not found. Automatic configuration seems to have failed.")
    else()
        message(FATAL_ERROR "Purple not found. Automatic configuration not available for this platform.")
    endif()
endif()
#find_package(Threads REQUIRED)
#find_package(PThreads4W REQUIRED)

message(STATUS "PURPLE_INCLUDE_DIRS: ${PURPLE_INCLUDE_DIRS}")
message(STATUS "PURPLE_LIBRARY_DIRS: ${PURPLE_LIBRARY_DIRS}")
message(STATUS "PURPLE_LIBRARIES: ${PURPLE_LIBRARIES}")

# find_path(QRENCODE_INCLUDE_DIR NAMES qrencode.h)
# find_library(QRENCODE_LIBRARIES qrencode)
# target_include_directories(main PRIVATE ${QRENCODE_INCLUDE_DIR})
# target_link_libraries(main PRIVATE ${QRENCODE_LIBRARIES})

include(FetchContent)
FetchContent_Declare( Corrosion GIT_REPOSITORY https://github.com/corrosion-rs/corrosion.git)
FetchContent_MakeAvailable(Corrosion)
corrosion_import_crate(MANIFEST_PATH src/rust/Cargo.toml)

set(TARGET_NAME purple-presage)
add_library(${TARGET_NAME} SHARED 
    "src/c/init.c"
    "src/c/bridge.c"
    "src/c/connection.c"
    "src/c/presage.h"
)
target_link_libraries(${TARGET_NAME} PRIVATE purple_presage_backend)

if (WIN32)
target_link_libraries(${TARGET_NAME} PRIVATE ws2_32.lib Crypt32.lib Ncrypt.lib Secur32.lib Userenv.lib NtDll.lib)
else()
target_link_libraries(purple_presage_backend INTERFACE m ${TARGET_NAME})
# additional libraries obtained via cargo rustc -- --print native-static-libs :
# gcc_s util rt pthread m dl c
endif()

#~ add_executable(test-presage 
    #~ "src/c/main.c"
#~ )
#~ target_link_libraries(test-presage PRIVATE purple_presage_backend)



file(READ "${CMAKE_SOURCE_DIR}/VERSION" PLUGIN_VERSION)
target_compile_definitions(${TARGET_NAME} PRIVATE PLUGIN_VERSION=${PLUGIN_VERSION})
target_include_directories(${TARGET_NAME} PRIVATE ${PURPLE_INCLUDE_DIRS})
target_link_libraries(${TARGET_NAME} PRIVATE ${PURPLE_LIBRARIES})
#target_link_libraries(${TARGET_NAME} PRIVATE Threads::Threads)
#target_link_libraries(${TARGET_NAME} PRIVATE PThreads4W::PThreads4W)
set_target_properties(${TARGET_NAME} PROPERTIES PREFIX "lib")

message(STATUS "PURPLE_PLUGIN_DIR: ${PURPLE_PLUGIN_DIR}")
if (WIN32)
    install(TARGETS ${TARGET_NAME} RUNTIME DESTINATION "${PURPLE_PLUGIN_DIR}")
else()
    install(TARGETS ${TARGET_NAME} DESTINATION "${PURPLE_PLUGIN_DIR}")
endif()

install(DIRECTORY "pixmaps" DESTINATION "${PURPLE_DATA_DIR}" FILES_MATCHING PATTERN "*.png")
