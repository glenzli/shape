//! Packaged Operator Workspace Host interaction checks.

#pragma once

#include <QString>

class QObject;
class DesktopBackend;
class UiPreferences;

namespace workspace_host_smoke {

bool verifySceneGraphRoutes(QObject& root_object);

bool verifyOperatorRoute(
    QObject& root_object,
    const QString& operator_type_key,
    const QString& expected_route,
    const QString& expected_workspace,
    const QString& expected_candidate_id = QString(),
    const QString& expected_child_object_name = QString(),
    const QString& expected_selected_tool_key = QString()
);

bool verifyLocalization(QObject& root_object, UiPreferences& ui_preferences);

bool verifyProjectWelcome(QObject& root_object);

bool verifyRecentProjects();

bool verifyOperatorDraftRoute(QObject& root_object, DesktopBackend& backend);

} // namespace workspace_host_smoke
