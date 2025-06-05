# package settings
# based on the ones from Bryant Eadon in purple-whatsmeow
# split version string into triple
# note: ${VERSION} is supposed to be set by parent
string(REPLACE "." ";" VERSION_LIST ${PLUGIN_VERSION})
list(GET VERSION_LIST 0 CPACK_PACKAGE_VERSION_MAJOR)
list(GET VERSION_LIST 1 CPACK_PACKAGE_VERSION_MINOR)
list(GET VERSION_LIST 2 CPACK_PACKAGE_VERSION_PATCH)
set(CPACK_PACKAGE_DESCRIPTION_SUMMARY "Signal protocol plug-in for libpurple")
set(CPACK_PACKAGE_VENDOR "Hermann Höhne")
set(CPACK_PACKAGE_DESCRIPTION "purple-presage is a Signal protocol plug-in for libpurple using the presage library.")
set(CPACK_PACKAGE_CONTACT "hoehermann@gmx.de")
# debian specific options
set(CPACK_GENERATOR "DEB" CACHE STRING "Which cpack generators to use.")
if (NOT ${CPACK_GENERATOR} STREQUAL "DEB")
    # CPACK_GENERATOR can be overridden on command-line
    message(WARNING "cpack genarator other than DEB has not been tested.")
endif()
set(CPACK_DEBIAN_PACKAGE_ARCHITECTURE "amd64") # TODO: use current architecture
set(CPACK_SOURCE_PACKAGE_FILE_NAME "${CMAKE_PROJECT_NAME}_${PLUGIN_VERSION}_${CPACK_DEBIAN_PACKAGE_ARCHITECTURE}")
set(CPACK_PACKAGE_FILE_NAME "${CMAKE_PROJECT_NAME}_${PLUGIN_VERSION}_${CPACK_DEBIAN_PACKAGE_ARCHITECTURE}")
set(CPACK_STRIP_FILES ON)
set(CPACK_DEBIAN_PACKAGE_DEPENDS "libpurple0 (>= ${PURPLE_VERSION}), libqrencode4") # TODO: libglib2.0-0 could be added here, but it is non-trivial to do. libpurple0 depends on it anyway, so we should be good.
set(CPACK_DEBIAN_PACKAGE_MAINTAINER "Hermann Höhne") #required

include(CPack)
