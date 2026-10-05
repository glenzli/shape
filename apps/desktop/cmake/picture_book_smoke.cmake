foreach(variable IN ITEMS SHAPE_BUILD_DIR SHAPE_DESKTOP_EXECUTABLE)
    if(NOT DEFINED ${variable} OR "${${variable}}" STREQUAL "")
        message(FATAL_ERROR "picture_book_smoke requires ${variable}")
    endif()
endforeach()
string(RANDOM LENGTH 12 ALPHABET 0123456789abcdef fixture_id)
set(fixture "${SHAPE_BUILD_DIR}/picture-book-smoke-${fixture_id}")
execute_process(
    COMMAND "${CMAKE_COMMAND}" -E env QT_QPA_PLATFORM=offscreen QT_QUICK_BACKEND=software
        "${SHAPE_DESKTOP_EXECUTABLE}" --smoke-picture-book "${fixture}"
    RESULT_VARIABLE result OUTPUT_VARIABLE output ERROR_VARIABLE error
    TIMEOUT 50
)
if(NOT result EQUAL 0)
    message(FATAL_ERROR "picture-book workflow failed; evidence ${fixture}: ${output} ${error}")
endif()
file(REMOVE_RECURSE "${fixture}")
message(STATUS "${output}")
