cmake_minimu_required(VERSION 3.2)
cmake_policy(VERSION 3.2)

include(mymacro.cmake)

# 项目名称
project( commlib_cxx_proj )

################################
# commlib_cxx_proj
################################
collect_cpp(. RAW_SRC_LIST)

# exclude
set(EXCLUDE_FILES
    debug_new.cpp
    dllmain.cpp
)
list(FILTER RAW_SRC_LIST EXCLUDE REGEX ".*_win32\.cpp$")
message( STATUS "==== commlib_cxx_proj exclude files ====" )
exclude_file_from(RAW_SRC_LIST EXCLUDE_FILES SRC_LIST)
message( STATUS "" )



#
include_directories(
    ${CMAKE_SOURCE_DIR}/src/libs/lua-5.1.5/src
    ${CMAKE_SOURCE_DIR}/src/libs/abseil-cpp
    ${CMAKE_SOURCE_DIR}/src/libs/protobuf/src
    ${CMAKE_SOURCE_DIR}/src/libs/flecs-3.1.4/include
    ${CMAKE_SOURCE_DIR}/src/libs/mariadb-connector-cpp-1.1.1/include
    ${CMAKE_SOURCE_DIR}/src/libs
)

get_directory_property(inc_dirs INCLUDE_DIRECTORIES)
message( STATUS "++++ commlib_cxx_proj include directories: ${inc_dirs}" )



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

get_directory_property(link_dirs LINK_DIRECTORIES)
message( STATUS "++++ commlib_cxx_proj link directories: ${link_dirs}" )


#link_libraries(
#    commlib_proto
#)

#
# lib
#
add_library(commlib STATIC ${SRC_LIST})

