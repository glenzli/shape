# Cargo publishes generated CXX headers below a hash-qualified OUT_DIR. Copy
# the one Shape-owned include tree into a stable build-local include root.

foreach(variable IN ITEMS
    SHAPE_CARGO_TARGET_DIRECTORY
    SHAPE_CARGO_PROFILE
    SHAPE_CXXBRIDGE_INCLUDE_DIRECTORY
)
    if(NOT DEFINED ${variable} OR "${${variable}}" STREQUAL "")
        message(FATAL_ERROR "sync_cxxbridge_headers requires ${variable}")
    endif()
endforeach()

file(
    GLOB candidate_directories
    LIST_DIRECTORIES true
    "${SHAPE_CARGO_TARGET_DIRECTORY}/${SHAPE_CARGO_PROFILE}/build/shape-desktop-bridge-*/out/cxxbridge/include"
)
set(valid_directories)
foreach(candidate_directory IN LISTS candidate_directories)
    if(EXISTS "${candidate_directory}/shape-desktop-bridge/src/lib.rs.h")
        list(APPEND valid_directories "${candidate_directory}")
    endif()
endforeach()
list(LENGTH valid_directories valid_directory_count)
if(NOT valid_directory_count EQUAL 1)
    message(FATAL_ERROR
        "expected one generated shape-desktop-bridge include directory, found ${valid_directory_count}"
    )
endif()

list(GET valid_directories 0 generated_include_directory)
file(MAKE_DIRECTORY "${SHAPE_CXXBRIDGE_INCLUDE_DIRECTORY}")
file(COPY "${generated_include_directory}/" DESTINATION "${SHAPE_CXXBRIDGE_INCLUDE_DIRECTORY}")
