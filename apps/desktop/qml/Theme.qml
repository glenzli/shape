pragma Singleton

import QtQuick

QtObject {
    enum AppearanceMode {
        System,
        Light,
        Dark
    }

    property int mode: Theme.AppearanceMode.System
    property bool systemDark: true
    readonly property bool effectiveDark: mode === Theme.AppearanceMode.Dark
                                          || (mode === Theme.AppearanceMode.System && systemDark)

    readonly property color background: effectiveDark ? "#0f1114" : "#f1f3f5"
    readonly property color chrome: effectiveDark ? "#121519" : "#fcfcfd"
    readonly property color surface: effectiveDark ? "#171a1f" : "#f8f9fb"
    readonly property color raised: effectiveDark ? "#1e2329" : "#ffffff"
    readonly property color raisedHover: effectiveDark ? "#252b32" : "#f3f5f7"
    readonly property color selected: effectiveDark ? "#202d39" : "#e9eef3"
    readonly property color border: effectiveDark ? "#323b45" : "#d9dee5"
    readonly property color borderStrong: effectiveDark ? "#485460" : "#c8d0d8"
    readonly property color text: effectiveDark ? "#f1f4f6" : "#20252a"
    readonly property color textSoft: effectiveDark ? "#cbd3d9" : "#4f5964"
    readonly property color muted: effectiveDark ? "#8d99a4" : "#707a85"
    readonly property color disabled: effectiveDark ? "#66737f" : "#a4acb5"
    readonly property color accent: effectiveDark ? "#70a3d6" : "#3574b9"
    readonly property color accentHover: effectiveDark ? "#82b2df" : "#2865a6"
    readonly property color accentSoft: effectiveDark ? "#223d58" : "#eaf2fa"
    readonly property color accentText: effectiveDark ? "#0d1117" : "#ffffff"
    readonly property color success: effectiveDark ? "#79b98a" : "#39764b"
    readonly property color danger: effectiveDark ? "#d77f79" : "#a5413d"
    readonly property color shadow: effectiveDark ? "#66000000" : "#22131b24"

    readonly property int controlHeight: 34
    readonly property int toolbarHeight: 44
    readonly property int radiusSmall: 7
    readonly property int radiusMedium: 10
    readonly property int radiusLarge: 14
    readonly property int fontMeta: 11
    readonly property int fontBody: 13
}
