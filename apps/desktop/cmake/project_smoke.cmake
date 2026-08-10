foreach(variable IN ITEMS
    SHAPE_SOURCE_DIR
    SHAPE_BUILD_DIR
    SHAPE_CARGO_EXECUTABLE
    SHAPE_DESKTOP_EXECUTABLE
)
    if(NOT DEFINED ${variable} OR "${${variable}}" STREQUAL "")
        message(FATAL_ERROR "project_smoke requires ${variable}")
    endif()
endforeach()

set(project_path "${SHAPE_BUILD_DIR}/shape-desktop-smoke-project.shape")
file(REMOVE_RECURSE "${project_path}")

execute_process(
    COMMAND
        "${SHAPE_CARGO_EXECUTABLE}" run --quiet --package shape-cli --
        demo "${project_path}"
    WORKING_DIRECTORY "${SHAPE_SOURCE_DIR}"
    RESULT_VARIABLE create_result
    OUTPUT_VARIABLE create_output
    ERROR_VARIABLE create_error
)
if(NOT create_result EQUAL 0)
    message(FATAL_ERROR "cannot create desktop smoke project: ${create_error}")
endif()

execute_process(
    COMMAND
        "${CMAKE_COMMAND}" -E env QT_QPA_PLATFORM=offscreen
        "${SHAPE_DESKTOP_EXECUTABLE}" --project "${project_path}"
        --smoke-text-cycle --smoke-raster-cycle --smoke-exit
    RESULT_VARIABLE desktop_result
    OUTPUT_VARIABLE desktop_output
    ERROR_VARIABLE desktop_error
)
if(NOT desktop_result EQUAL 0)
    file(REMOVE_RECURSE "${project_path}")
    message(FATAL_ERROR
        "desktop failed its text candidate cycle: ${desktop_error}"
    )
endif()

execute_process(
    COMMAND
        "${CMAKE_COMMAND}" -E env QT_QPA_PLATFORM=offscreen
        "${SHAPE_DESKTOP_EXECUTABLE}" --project "${project_path}" --smoke-exit
    RESULT_VARIABLE reopen_result
    OUTPUT_VARIABLE reopen_output
    ERROR_VARIABLE reopen_error
)
file(REMOVE_RECURSE "${project_path}")
if(NOT reopen_result EQUAL 0)
    message(FATAL_ERROR
        "desktop failed to reopen its accepted text revision: ${reopen_error}"
    )
endif()
