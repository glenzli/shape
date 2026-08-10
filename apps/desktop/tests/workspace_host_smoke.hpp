//! Packaged Operator Workspace Host interaction checks.

#pragma once

#include <QString>

class QObject;
class UiPreferences;

namespace workspace_host_smoke {

bool verifySceneGraphRoutes(QObject& root_object);

bool verifyOperatorRoute(
    QObject& root_object,
    const QString& operator_type_key,
    const QString& expected_route,
    const QString& expected_workspace,
    const QString& expected_candidate_id = QString()
);

bool verifyLocalization(QObject& root_object, UiPreferences& ui_preferences);

} // namespace workspace_host_smoke
