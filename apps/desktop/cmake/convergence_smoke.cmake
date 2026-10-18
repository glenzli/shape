foreach(variable IN ITEMS SHAPE_SOURCE_DIR SHAPE_BUILD_DIR SHAPE_CARGO_EXECUTABLE SHAPE_DESKTOP_EXECUTABLE SHAPE_CARGO_TARGET_DIRECTORY)
    if(NOT DEFINED ${variable} OR "${${variable}}" STREQUAL "")
        message(FATAL_ERROR "convergence_smoke requires ${variable}")
    endif()
endforeach()
string(TIMESTAMP stamp "%Y%m%dT%H%M%S")
string(RANDOM LENGTH 6 ALPHABET abcdef0123456789 nonce)
set(evidence "${SHAPE_BUILD_DIR}/convergence-smoke-${stamp}-${nonce}")
file(MAKE_DIRECTORY "${evidence}")
if(NOT SHAPE_COMPAT_FIXTURE_BUNDLE)
    set(factory "${evidence}/factory")
    execute_process(COMMAND "${CMAKE_COMMAND}" -E env
        "SHAPE_VOX_FIXTURE_ROOT=${factory}" "CARGO_TARGET_DIR=${SHAPE_CARGO_TARGET_DIRECTORY}"
        "${SHAPE_CARGO_EXECUTABLE}" test --quiet --package shape-core
        full_legacy_transfer_preserves_unaccepted_text_voices_cues_and_historical_audio_source
        WORKING_DIRECTORY "${SHAPE_SOURCE_DIR}" RESULT_VARIABLE factory_status)
    if(NOT factory_status EQUAL 0)
        message(FATAL_ERROR "legacy test-only factory failed")
    endif()
    set(SHAPE_COMPAT_FIXTURE_BUNDLE "${factory}/legacy.shape")
endif()
file(COPY "${SHAPE_COMPAT_FIXTURE_BUNDLE}" DESTINATION "${evidence}")
get_filename_component(bundle_name "${SHAPE_COMPAT_FIXTURE_BUNDLE}" NAME)
execute_process(COMMAND "${CMAKE_COMMAND}" -E env
    QT_QPA_PLATFORM=offscreen QT_QUICK_BACKEND=software QTWEBENGINE_DISABLE_SANDBOX=1
    "${SHAPE_DESKTOP_EXECUTABLE}" --project "${evidence}/${bundle_name}" --smoke-convergence "${evidence}/results"
    RESULT_VARIABLE result OUTPUT_VARIABLE output ERROR_VARIABLE errors)
file(WRITE "${evidence}/native.log" "${output}\n${errors}")
message(STATUS "convergence evidence: ${evidence}")
if(NOT result EQUAL 0 OR "${errors}" MATCHES "QQml|ReferenceError|TypeError|Binding loop|failed to load")
    message(FATAL_ERROR "convergence smoke failed: ${errors}")
endif()
