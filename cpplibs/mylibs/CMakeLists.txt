cmake_minimu_required(VERSION 3.2)
cmake_policy(VERSION 3.2)

include(mymacro.cmake)

if (CMAKE_BUILD_TYPE MATCHES "Debug" OR CMAKE_BUILD_TYPE MATCHES "None")
    set(CMAKE_ARCHIVE_OUTPUT_DIRECTORY ${CMAKE_SOURCE_DIR}/libs/centos/Debug)
    set(CMAKE_LIBRARY_OUTPUT_DIRECTORY ${CMAKE_SOURCE_DIR}/libs/centos/Debug)
    set(CMAKE_RUNTIME_OUTPUT_DIRECTORY ${CMAKE_SOURCE_DIR}/bin/Debug)
elseif (CMAKE_BUILD_TYPE MATCHES "Release")
    set(CMAKE_ARCHIVE_OUTPUT_DIRECTORY ${CMAKE_SOURCE_DIR}/libs/centos/Release)
    set(CMAKE_LIBRARY_OUTPUT_DIRECTORY ${CMAKE_SOURCE_DIR}/libs/centos/Release)
    set(CMAKE_RUNTIME_OUTPUT_DIRECTORY ${CMAKE_SOURCE_DIR}/bin)
endif()
message( STATUS "CMAKE_ARCHIVE_OUTPUT_DIRECTORY = ${CMAKE_ARCHIVE_OUTPUT_DIRECTORY}" )
message( STATUS "CMAKE_ARCHIVE_OUTPUT_DIRECTORY = ${CMAKE_ARCHIVE_OUTPUT_DIRECTORY}" )
message( STATUS "CMAKE_RUNTIME_OUTPUT_DIRECTORY = ${CMAKE_RUNTIME_OUTPUT_DIRECTORY}" )



# 项目名称
project( mylibs_proj )
################################
# mylibs_proj
################################

if (CMAKE_BUILD_TYPE MATCHES "Debug" OR CMAKE_BUILD_TYPE MATCHES "None")
    set(CMAKE_CXX_FLAGS "${CMAKE_CXX_FLAGS} -g -O0)
elseif (CMAKE_BUILD_TYPE MATCHES "Release")
    set(CMAKE_CXX_FLAGS "${CMAKE_CXX_FLAGS} -g -O2)
endif()

set(CMAKE_CXX_FLAGS "${CMAKE_CXX_FLAGS} -Wall -Wno-deprecated -Wno-unknown-pragmas -lssl -lcrypto -ldl -lrt -lresolv -pthread)
add_definitions("-fPIC")
add_definitions(-DUNIX -DUNICODE -D_UNICODE -D_REENTRANT -D__STDC_FORMAT_MACROS -DCURL_STATICLIB)
add_definitions("-fstack-protector-all")
#add_definitions("-flarge-source-files")

if (ASAN_OPEN MATCHES "Yes")
    set(CMAKE_CXX_FLAGS "${CMAKE_CXX_FLAGS} -lasan")

    add_definitions("-fsanitize=address")
    add_definitions("-fsanitize=leak")
    #add_definitions("-fno-var-tracking")
    add_definitions("-fno-omit-frame-pointer")
    add_definitions("-ggdb")

else()

endif()


#set(CMAKE_CXX_CLANG_TIDY clang-tidy;)


message( STATUS "CMAKE_BUILD_TYPE ======== ======== ${CMAKE_BUILD_TYPE} ======== ========" )
message( STATUS "ASAN_OPEN = ${ASAN_OPEN}" )

message( STATUS "CMAKE_C_FLAGS = ${CMAKE_C_FLAGS}" )
message( STATUS "CMAKE_CXX_FLAGS = ${CMAKE_CXX_FLAGS}" )

get_directory_property(compile_defs COMPILE_DEFINITIONS)
message( STATUS "Compile definitions: ${compile_defs}" )



#
include_directories(
    ${CMAKE_SOURCE_DIR}/src/libs/spdlog-1.11.0/include
    ${CMAKE_SOURCE_DIR}/src/libs/openssl-3/include
    ${CMAKE_SOURCE_DIR}/src/libs/curl/include
    ${CMAKE_SOURCE_DIR}/src/libs/libevent-2.1.12-stable/include
    ${CMAKE_SOURCE_DIR}/src/libs/readline-8.1
)



# debug or release
if (CMAKE_BUILD_TYPE MATCHES "Debug" OR CMAKE_BUILD_TYPE MATCHES "None")
    link_directories(
        ${CMAKE_SOURCE_DIR}/libs/centos/Debug
    )
elseif (CMAKE_BUILD_TYPE MATCHES "Release")
    link_directories(
        ${CMAKE_SOURCE_DIR}/libs/centos/Release
    )
endif()



if (CMAKE_BUILD_TYPE MATCHES "Debug" OR CMAKE_BUILD_TYPE MATCHES "None")
    link_libraries(
        ${CMAKE_SOURCE_DIR}/libs/centos/Debug/libssl.a
        ${CMAKE_SOURCE_DIR}/libs/centos/Debug/libcrypto.a
        ${CMAKE_SOURCE_DIR}/libs/centos/Debug/libcurl.a
        ${CMAKE_SOURCE_DIR}/libs/centos/Debug/libevent.a
        ${CMAKE_SOURCE_DIR}/libs/centos/Debug/libevent_openssl.a
        ${CMAKE_SOURCE_DIR}/libs/centos/Debug/libz.a
    )
elseif (CMAKE_BUILD_TYPE MATCHES "Release")
    link_libraries(
        ${CMAKE_SOURCE_DIR}/libs/centos/Release/libssl.a
        ${CMAKE_SOURCE_DIR}/libs/centos/Release/libcrypto.a
        ${CMAKE_SOURCE_DIR}/libs/centos/Release/libcurl.a
        ${CMAKE_SOURCE_DIR}/libs/centos/Release/libevent.a
        ${CMAKE_SOURCE_DIR}/libs/centos/Release/libevent_openssl.a
        ${CMAKE_SOURCE_DIR}/libs/centos/Release/libz.a
    )
endif()

#
# libs
#
add_subdirectory(${CMAKE_SOURCE_DIR}/src/commlib_cxx)

