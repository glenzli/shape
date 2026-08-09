find_package(Qt6 6.5 REQUIRED COMPONENTS Quick QuickControls2 LinguistTools)

qt_standard_project_setup(REQUIRES 6.5 I18N_TRANSLATED_LANGUAGES zh_CN)
qt_policy(SET QTP0004 NEW)

qt_add_executable(shape-desktop
    src/main.cpp
)

qt_add_qml_module(shape-desktop
    URI Shape.Desktop
    VERSION 1.0
    QML_FILES
        qml/Main.qml
        qml/Theme.qml
        qml/components/ArtifactWorkspace.qml
        qml/components/IntentPanel.qml
        qml/components/SemanticHistory.qml
        qml/components/VariantsPanel.qml
)

qt_add_translations(shape-desktop
    TS_FILES translations/shape_zh_CN.ts
    RESOURCE_PREFIX "/translations"
)

target_compile_features(shape-desktop PRIVATE cxx_std_20)

target_link_libraries(shape-desktop
    PRIVATE
        Qt6::Quick
        Qt6::QuickControls2
)

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
        TIMEOUT 15
    )
endif()
