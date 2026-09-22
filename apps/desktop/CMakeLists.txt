find_package(Qt6 6.9 REQUIRED COMPONENTS Concurrent Multimedia Quick QuickControls2 LinguistTools)
find_package(Threads REQUIRED)

qt_standard_project_setup(REQUIRES 6.9 I18N_TRANSLATED_LANGUAGES zh_CN)
qt_policy(SET QTP0004 NEW)

# Cargo owns the Rust project reader and generated CXX ABI. CMake owns the
# final Qt executable and links one production consumer against the archive.
find_program(SHAPE_CARGO_EXECUTABLE cargo REQUIRED)
if(CMAKE_BUILD_TYPE STREQUAL "Release")
    set(SHAPE_CARGO_PROFILE release)
    set(SHAPE_CARGO_PROFILE_ARGUMENT --release)
else()
    set(SHAPE_CARGO_PROFILE debug)
    set(SHAPE_CARGO_PROFILE_ARGUMENT)
endif()

set(SHAPE_CARGO_TARGET_DIRECTORY "${CMAKE_CURRENT_BINARY_DIR}/cargo-target")
if(WIN32)
    set(SHAPE_DESKTOP_RUST_LIBRARY
        "${SHAPE_CARGO_TARGET_DIRECTORY}/${SHAPE_CARGO_PROFILE}/shape_desktop_bridge.lib"
    )
else()
    set(SHAPE_DESKTOP_RUST_LIBRARY
        "${SHAPE_CARGO_TARGET_DIRECTORY}/${SHAPE_CARGO_PROFILE}/libshape_desktop_bridge.a"
    )
endif()
set(SHAPE_CXXBRIDGE_INCLUDE_DIRECTORY "${SHAPE_CARGO_TARGET_DIRECTORY}/cxxbridge")

file(
    GLOB_RECURSE SHAPE_RUST_BRIDGE_INPUTS
    CONFIGURE_DEPENDS
    "${PROJECT_SOURCE_DIR}/Cargo.toml"
    "${PROJECT_SOURCE_DIR}/Cargo.lock"
    "${PROJECT_SOURCE_DIR}/crates/*/Cargo.toml"
    "${PROJECT_SOURCE_DIR}/crates/*/build.rs"
    "${PROJECT_SOURCE_DIR}/crates/*/src/*.rs"
    "${PROJECT_SOURCE_DIR}/docs/SPEECH_SCRIPT_PROMPT.md"
)

if(UNIX)
    find_program(SHAPE_ENV_EXECUTABLE env REQUIRED)
    set(SHAPE_CARGO_ENV "${SHAPE_ENV_EXECUTABLE}")
else()
    set(SHAPE_CARGO_ENV "${CMAKE_COMMAND}" -E env)
endif()
if(APPLE)
    # Homebrew Qt 6.11 is currently built for macOS 26.0. Match the Rust
    # archive so one final link does not mix deployment targets.
    list(APPEND SHAPE_CARGO_ENV MACOSX_DEPLOYMENT_TARGET=26.0)
endif()

add_custom_command(
    OUTPUT "${SHAPE_DESKTOP_RUST_LIBRARY}"
    BYPRODUCTS
        "${SHAPE_CXXBRIDGE_INCLUDE_DIRECTORY}/shape-desktop-bridge/src/lib.rs.h"
    COMMAND
        ${SHAPE_CARGO_ENV} "${SHAPE_CARGO_EXECUTABLE}" build
        --package shape-desktop-bridge
        ${SHAPE_CARGO_PROFILE_ARGUMENT}
        --target-dir "${SHAPE_CARGO_TARGET_DIRECTORY}"
    COMMAND
        "${CMAKE_COMMAND}"
        "-DSHAPE_CARGO_TARGET_DIRECTORY=${SHAPE_CARGO_TARGET_DIRECTORY}"
        "-DSHAPE_CARGO_PROFILE=${SHAPE_CARGO_PROFILE}"
        "-DSHAPE_CXXBRIDGE_INCLUDE_DIRECTORY=${SHAPE_CXXBRIDGE_INCLUDE_DIRECTORY}"
        -P "${CMAKE_CURRENT_SOURCE_DIR}/cmake/sync_cxxbridge_headers.cmake"
    DEPENDS ${SHAPE_RUST_BRIDGE_INPUTS}
    WORKING_DIRECTORY "${PROJECT_SOURCE_DIR}"
    COMMENT "Building the Rust desktop bridge"
    JOB_SERVER_AWARE TRUE
    VERBATIM
)
add_custom_target(shape-desktop-rust-build DEPENDS "${SHAPE_DESKTOP_RUST_LIBRARY}")
add_library(shape-desktop-rust STATIC IMPORTED GLOBAL)
set_target_properties(
    shape-desktop-rust
    PROPERTIES IMPORTED_LOCATION "${SHAPE_DESKTOP_RUST_LIBRARY}"
)
add_dependencies(shape-desktop-rust shape-desktop-rust-build)

qt_add_executable(shape-desktop
    src/main.cpp
    src/desktop_backend.cpp
    src/desktop_backend.hpp
    src/infer_runtime_controller.cpp
    src/infer_runtime_controller.hpp
    src/infer_text_controller.cpp
    src/infer_text_controller.hpp
    src/infer_speech_controller.cpp
    src/infer_speech_controller.hpp
    src/infer_image_controller.cpp
    src/infer_image_controller.hpp
    src/infer_controller_support.hpp
    src/audio_export_controller.cpp
    src/audio_export_controller.hpp
    src/audio_preview_controller.cpp
    src/audio_preview_controller.hpp
    src/image_preview_provider.cpp
    src/image_preview_provider.hpp
    src/ui_preferences.cpp
    src/ui_preferences.hpp
    src/recent_projects.cpp
    src/recent_projects.hpp
    src/speech_script_documentation.cpp
    src/speech_script_documentation.hpp
    src/text_expression_library.cpp
    src/text_expression_library.hpp
    src/mac_titlebar.hpp
    tests/workspace_host_smoke.cpp
    tests/workspace_host_smoke.hpp
    tests/speech_script_help_smoke.cpp
    tests/speech_script_help_smoke.hpp
    tests/image_live_smoke.cpp
    tests/image_live_smoke.hpp
    tests/text_authoring_smoke.cpp
    tests/text_authoring_smoke.hpp
    tests/speech_live_smoke.cpp
    tests/speech_live_smoke.hpp
)
if(APPLE)
    target_sources(shape-desktop PRIVATE src/mac_titlebar.mm)
endif()

set_source_files_properties(qml/Theme.qml PROPERTIES QT_QML_SINGLETON_TYPE TRUE)
qt_add_qml_module(shape-desktop
    URI Shape.Desktop
    VERSION 1.0
    QML_FILES
        qml/Main.qml
        qml/BranchArtifactDialog.qml
        qml/MainTitleBar.qml
        qml/ShapeSettingsDialog.qml
        qml/Theme.qml
        qml/components/ArtifactDetailsPanel.qml
        qml/components/ArtifactLineagePanel.qml
        qml/components/AudioExportDialog.qml
        qml/components/SpeechScriptHelpDialog.qml
        qml/components/SpeechScriptPanel.qml
        qml/components/AudioSpeechOperatorWorkspace.qml
        qml/components/AiImageOperatorWorkspace.qml
        qml/components/CandidateFilmstrip.qml
        qml/components/ImageCompareWorkspace.qml
        qml/components/ContextInspector.qml
        qml/components/CreateProjectDialog.qml
        qml/components/CreateSceneTypeDialog.qml
        qml/components/CreateAiImageSceneDialog.qml
        qml/components/CreateTextSceneDialog.qml
        qml/components/CreativeCandidateNodeCard.qml
        qml/components/CreativeDraftNodeCard.qml
        qml/components/CreativeGraphNodeCard.qml
        qml/components/GraphSelectionInspector.qml
        qml/components/ImageEditorWorkspace.qml
        qml/components/OperatorPalette.qml
        qml/components/OperatorIntentSidebar.qml
        qml/components/OperatorWorkspaceHost.qml
        qml/components/RasterCropOperatorWorkspace.qml
        qml/components/RasterEffectsWorkspace.qml
        qml/components/RasterResizeOperatorWorkspace.qml
        qml/components/SceneGraphToolbar.qml
        qml/components/SceneGraphContextStrip.qml
        qml/components/SceneOperatorGraphWorkspace.qml
        qml/components/ProjectNavigator.qml
        qml/components/ProjectWelcome.qml
        qml/components/ReadOnlyNodeWorkspace.qml
        qml/components/ShapeButton.qml
        qml/components/ShapeTextField.qml
        qml/components/ShapeTextArea.qml
        qml/components/ShapeTextEditor.qml
        qml/components/ShapeComboBox.qml
        qml/components/ShapeDialog.qml
        qml/components/TextAuthoringWorkspace.qml
        qml/components/ScriptPresentation.js
        qml/components/TextFormatPanel.qml
        qml/components/SpeechScriptReadingView.qml
        qml/components/ShapeIcon.qml
        qml/components/ShapeIconButton.qml
        qml/components/SourceMaterialWorkspace.qml
        qml/components/TextCompareWorkspace.qml
        qml/components/TextExpressionPalette.qml
        qml/components/VariantsPanel.qml
        qml/components/WorkbenchProjectRail.qml
        qml/components/WorkGraphProjection.js
        qml/components/WorkspaceSurface.qml
    RESOURCES
        icons/add.svg
        icons/back.svg
        icons/close.svg
        icons/compare.svg
        icons/crop.svg
        icons/edit.svg
        icons/export.svg
        icons/fit.svg
        icons/open.svg
        icons/search.svg
        icons/settings.svg
        icons/sparkle.svg
        icons/trash.svg
        icons/verified.svg
        icons/waveform.svg
        icons/zoom-in.svg
        icons/zoom-out.svg
)

qt_add_resources(shape-desktop speech-script-documentation
    PREFIX "/shape/help"
    BASE "${PROJECT_SOURCE_DIR}/docs"
    FILES
        "${PROJECT_SOURCE_DIR}/docs/SPEECH_SCRIPT.md"
        "${PROJECT_SOURCE_DIR}/docs/SPEECH_SCRIPT_PROMPT.md"
)

qt_add_translations(shape-desktop
    TS_FILES translations/shape_zh_CN.ts
    RESOURCE_PREFIX "/translations"
    LUPDATE_OPTIONS -no-obsolete
    LRELEASE_OPTIONS -nounfinished
)

target_compile_features(shape-desktop PRIVATE cxx_std_20)

target_include_directories(shape-desktop
    PRIVATE
        "${CMAKE_CURRENT_SOURCE_DIR}/src"
        "${CMAKE_CURRENT_SOURCE_DIR}/tests"
        "${SHAPE_CXXBRIDGE_INCLUDE_DIRECTORY}"
)

target_link_libraries(shape-desktop
    PRIVATE
        Qt6::Quick
        Qt6::QuickControls2
        Qt6::Concurrent
        Qt6::Multimedia
        shape-desktop-rust
        Threads::Threads
        ${CMAKE_DL_LIBS}
)
add_dependencies(shape-desktop shape-desktop-rust-build)

if(MSVC)
    target_compile_options(shape-desktop PRIVATE /W4 /permissive-)
else()
    target_compile_options(shape-desktop PRIVATE -Wall -Wextra -Wpedantic)
endif()

set_target_properties(shape-desktop PROPERTIES
    MACOSX_BUNDLE TRUE
    WIN32_EXECUTABLE TRUE
    MACOSX_BUNDLE_BUNDLE_NAME "Shape"
    MACOSX_BUNDLE_GUI_IDENTIFIER "dev.shape.desktop"
)

install(TARGETS shape-desktop
    BUNDLE DESTINATION .
    RUNTIME DESTINATION bin
)

if(BUILD_TESTING)
    add_test(
        NAME shape-desktop-smoke
        COMMAND $<TARGET_FILE:shape-desktop> --smoke-exit
    )
    set_tests_properties(shape-desktop-smoke PROPERTIES
        ENVIRONMENT "QT_QPA_PLATFORM=offscreen"
        FAIL_REGULAR_EXPRESSION "QQml;ReferenceError;TypeError;Binding loop;failed to load"
        TIMEOUT 15
    )
    add_test(
        NAME shape-desktop-project-smoke
        COMMAND
            "${CMAKE_COMMAND}"
            "-DSHAPE_SOURCE_DIR=${PROJECT_SOURCE_DIR}"
            "-DSHAPE_BUILD_DIR=${CMAKE_CURRENT_BINARY_DIR}"
            "-DSHAPE_CARGO_EXECUTABLE=${SHAPE_CARGO_EXECUTABLE}"
            "-DSHAPE_DESKTOP_EXECUTABLE=$<TARGET_FILE:shape-desktop>"
            -P "${CMAKE_CURRENT_SOURCE_DIR}/cmake/project_smoke.cmake"
    )
    set_tests_properties(shape-desktop-project-smoke PROPERTIES
        FAIL_REGULAR_EXPRESSION "QQml;ReferenceError;TypeError;Binding loop;failed to load"
        TIMEOUT 60
    )
endif()
