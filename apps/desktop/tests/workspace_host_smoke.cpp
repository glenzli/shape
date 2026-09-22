#include "workspace_host_smoke.hpp"
#include "speech_script_help_smoke.hpp"
#include <QJsonDocument>
#include <QJsonObject>

#include "desktop_backend.hpp"
#include "ui_preferences.hpp"

#include <QCoreApplication>
#include <QMetaObject>
#include <QObject>
#include <QQmlEngine>
#include <QQmlExpression>
#include <QQuickItem>
#include <QUrl>
#include <QVariant>

#include <algorithm>
#include <iostream>
#include <optional>

namespace {

std::optional<QVariantMap>
graph_node_with_role(const QVariantList& nodes, const QString& role_key) {
    const auto node =
        std::find_if(nodes.cbegin(), nodes.cend(), [&role_key](const QVariant& value) {
            return value.toMap().value(QStringLiteral("roleKey")).toString() == role_key;
        });
    if (node == nodes.cend()) {
        return std::nullopt;
    }
    return node->toMap();
}

std::optional<QVariantMap>
graph_operator_with_type(const QVariantList& nodes, const QString& operator_type_key) {
    const auto node =
        std::find_if(nodes.cbegin(), nodes.cend(), [&operator_type_key](const QVariant& value) {
            const QVariantMap projected = value.toMap();
            return projected.value(QStringLiteral("roleKey")).toString()
                       == QStringLiteral("operator")
                   && projected.value(QStringLiteral("operatorTypeKey")).toString()
                          == operator_type_key;
        });
    if (node == nodes.cend()) {
        return std::nullopt;
    }
    return node->toMap();
}

QQuickItem* find_quick_item(QQuickItem* root, const QString& object_name) {
    if (root == nullptr) {
        return nullptr;
    }
    if (root->objectName() == object_name) {
        return root;
    }
    for (QQuickItem* const child : root->childItems()) {
        if (QQuickItem* const match = find_quick_item(child, object_name); match != nullptr) {
            return match;
        }
    }
    return nullptr;
}

bool invoke_packaged_click(QObject& root, const QString& object_name, const char* failure) {
    QObject* target = find_quick_item(qobject_cast<QQuickItem*>(&root), object_name);
    if (target == nullptr) {
        target = root.findChild<QObject*>(object_name);
    }
    if (target == nullptr) {
        std::cerr << failure << ": object not found: " << object_name.toStdString() << std::endl;
        return false;
    }
    if (!QMetaObject::invokeMethod(target, "clicked", Qt::DirectConnection)) {
        std::cerr << failure << ": clicked signal is not invokable" << std::endl;
        return false;
    }
    QCoreApplication::processEvents();
    return true;
}

bool verify_packaged_node_route(
    QObject& workspace_surface,
    const QVariantMap& node,
    const QString& expected_route,
    const QString& expected_workspace,
    const QString& expected_candidate_id = QString(),
    const QString& expected_child_object_name = QString(),
    const QString& expected_selected_tool_key = QString()
) {
    const QString node_id = node.value(QStringLiteral("id")).toString();
    const QVariantList graph_nodes = workspace_surface.property("graphNodes").toList();
    const auto selected_node =
        std::find_if(graph_nodes.cbegin(), graph_nodes.cend(), [&node_id](const QVariant& value) {
            return value.toMap().value(QStringLiteral("id")).toString() == node_id;
        });
    if (selected_node == graph_nodes.cend()) {
        std::cerr << "desktop graph smoke could not resolve the packaged node index" << std::endl;
        return false;
    }
    const auto node_index = std::distance(graph_nodes.cbegin(), selected_node);
    if (!invoke_packaged_click(
            workspace_surface,
            QStringLiteral("acceptedGraphNode-%1").arg(node_index),
            "desktop graph smoke could not click a packaged node"
        )
        || workspace_surface.property("selectedNodeId").toString() != node_id
        || workspace_surface.property("currentMode").toInt() != 0) {
        std::cerr << "desktop graph smoke selection opened a workspace" << std::endl;
        return false;
    }
    if (!invoke_packaged_click(
            workspace_surface,
            QStringLiteral("openSelectedNodeButton"),
            "desktop graph smoke could not click the packaged open action"
        )) {
        return false;
    }

    QObject* const host =
        workspace_surface.findChild<QObject*>(QStringLiteral("operatorWorkspaceHost"));
    if (host == nullptr || workspace_surface.property("currentMode").toInt() != 1
        || !workspace_surface.property("focusActive").toBool()
        || workspace_surface.property("workspaceRouteKey").toString() != expected_route
        || workspace_surface.property("loadedWorkspaceObjectName").toString() != expected_workspace
        || host->property("openedNodeId").toString() != node_id
        || host->property("openedArtifactId").toString()
               != node.value(QStringLiteral("artifactId")).toString()
        || host->property("openedRevisionId").toString()
               != node.value(QStringLiteral("revisionId")).toString()
        || host->property("openedTransformationId").toString()
               != node.value(QStringLiteral("transformationId")).toString()
        || host->property("selectedCandidateId").toString() != expected_candidate_id) {
        std::cerr << "desktop graph smoke entered the wrong packaged node workspace" << std::endl;
        return false;
    }

    QObject* const loaded_workspace = qvariant_cast<QObject*>(host->property("loadedWorkspace"));
    if ((!expected_child_object_name.isEmpty()
         && (loaded_workspace == nullptr
             || loaded_workspace->findChild<QObject*>(expected_child_object_name) == nullptr))
        || (!expected_selected_tool_key.isEmpty()
            && (loaded_workspace == nullptr
                || loaded_workspace->property("selectedToolKey").toString()
                       != expected_selected_tool_key))) {
        std::cerr << "desktop graph smoke entered an incomplete packaged node workspace"
                  << std::endl;
        return false;
    }

    if (expected_route == QStringLiteral("source.readonly")) {
        QObject* const source_text =
            host->findChild<QObject*>(QStringLiteral("sourceMaterialText"));
        if (!node.value(QStringLiteral("hasTextPreview")).toBool() || source_text == nullptr
            || source_text->property("text").toString()
                   != node.value(QStringLiteral("textPreview")).toString()) {
            std::cerr << "desktop graph smoke did not show the exact Source revision content"
                      << std::endl;
            return false;
        }
    }

    if (!invoke_packaged_click(
            workspace_surface,
            QStringLiteral("returnToSceneGraphButton"),
            "desktop graph smoke could not click the packaged return action"
        )
        || workspace_surface.property("currentMode").toInt() != 0
        || !workspace_surface.property("graphActive").toBool()
        || workspace_surface.property("selectedNodeId").toString() != node_id
        || host->property("active").toBool()) {
        std::cerr << "desktop graph smoke did not preserve selection on return" << std::endl;
        return false;
    }
    return true;
}

} // namespace

namespace workspace_host_smoke {

bool verifyOperatorRoute(
    QObject& root_object,
    const QString& operator_type_key,
    const QString& expected_route,
    const QString& expected_workspace,
    const QString& expected_candidate_id,
    const QString& expected_child_object_name,
    const QString& expected_selected_tool_key
) {
    QObject* const workspace_surface =
        root_object.findChild<QObject*>(QStringLiteral("workspaceSurface"));
    if (workspace_surface == nullptr
        || !QMetaObject::invokeMethod(workspace_surface, "showGraph", Qt::DirectConnection)) {
        std::cerr << "desktop graph smoke could not restore the Scene Graph" << std::endl;
        return false;
    }
    QCoreApplication::processEvents();
    const auto operator_node = graph_operator_with_type(
        workspace_surface->property("graphNodes").toList(),
        operator_type_key
    );
    return operator_node.has_value()
           && verify_packaged_node_route(
               *workspace_surface,
               *operator_node,
               expected_route,
               expected_workspace,
               expected_candidate_id,
               expected_child_object_name,
               expected_selected_tool_key
           );
}

bool verifySceneGraphRoutes(QObject& root_object) {
    QObject* const workspace_surface =
        root_object.findChild<QObject*>(QStringLiteral("workspaceSurface"));
    if (workspace_surface == nullptr || workspace_surface->property("currentMode").toInt() != 0
        || !workspace_surface->property("graphActive").toBool()) {
        std::cerr << "desktop graph smoke did not start at the Scene Graph" << std::endl;
        return false;
    }

    QQmlExpression activation(
        QQmlEngine::contextForObject(workspace_surface),
        workspace_surface,
        QStringLiteral("activateScene(1)")
    );
    activation.evaluate();
    if (activation.hasError()) {
        std::cerr << "desktop graph smoke could not activate the branch Scene: "
                  << activation.error().toString().toStdString() << std::endl;
        return false;
    }
    QCoreApplication::processEvents();
    if (root_object.property("selectedArtifactIndex").toInt() != 1) {
        std::cerr << "desktop graph smoke did not synchronize Scene selection" << std::endl;
        return false;
    }
    if (workspace_surface->property("currentMode").toInt() != 0) {
        std::cerr << "desktop graph smoke left the graph during Scene selection" << std::endl;
        return false;
    }

    const QVariantList graph_nodes = workspace_surface->property("graphNodes").toList();
    const auto source_node = graph_node_with_role(graph_nodes, QStringLiteral("source"));
    const auto operator_node = graph_node_with_role(graph_nodes, QStringLiteral("operator"));
    const auto output_node = graph_node_with_role(graph_nodes, QStringLiteral("output"));
    if (graph_nodes.size() != 3 || !source_node.has_value() || !operator_node.has_value()
        || !output_node.has_value()
        || !source_node->value(QStringLiteral("hasTextPreview")).toBool()
        || source_node->value(QStringLiteral("textPreview")).toString().isEmpty()
        || source_node->value(QStringLiteral("textPreview")).toString()
               == output_node->value(QStringLiteral("textPreview")).toString()
        || operator_node->value(QStringLiteral("operatorTypeKey")).toString()
               != QStringLiteral("text.edit")
        || operator_node->value(QStringLiteral("artifactId")).toString().isEmpty()
        || operator_node->value(QStringLiteral("revisionId")).toString().isEmpty()
        || operator_node->value(QStringLiteral("transformationId")).toString().isEmpty()) {
        std::cerr << "desktop graph smoke did not project Source -> Text Edit -> Output"
                  << std::endl;
        return false;
    }
    if (!verify_packaged_node_route(
            *workspace_surface,
            *source_node,
            QStringLiteral("source.readonly"),
            QStringLiteral("sourceMaterialWorkspace")
        )
        || !verify_packaged_node_route(
            *workspace_surface,
            *operator_node,
            QStringLiteral("operator.text.edit"),
            QStringLiteral("textAuthoringWorkspace")
        )
        || !verify_packaged_node_route(
            *workspace_surface,
            *output_node,
            QStringLiteral("output.readonly"),
            QStringLiteral("outputReadOnlyWorkspace")
        )) {
        return false;
    }

    QObject* const host =
        workspace_surface->findChild<QObject*>(QStringLiteral("operatorWorkspaceHost"));
    if (host == nullptr) {
        std::cerr << "desktop graph smoke could not find the packaged workspace host" << std::endl;
        return false;
    }
    QQmlExpression reopen_text_operator(
        QQmlEngine::contextForObject(workspace_surface),
        workspace_surface,
        QStringLiteral("openNode('%1')").arg(operator_node->value(QStringLiteral("id")).toString())
    );
    if (!reopen_text_operator.evaluate().toBool() || reopen_text_operator.hasError()) {
        std::cerr << "desktop graph smoke could not reopen the text Operator" << std::endl;
        return false;
    }
    QQmlExpression open_speech_workspace(
        QQmlEngine::contextForObject(workspace_surface),
        workspace_surface,
        QStringLiteral("openSpeechWorkspace()")
    );
    if (!open_speech_workspace.evaluate().toBool() || open_speech_workspace.hasError()
        || workspace_surface->property("workspaceRouteKey").toString()
               != QStringLiteral("operator.audio.speech_synthesize")
        || workspace_surface->property("loadedWorkspaceObjectName").toString()
               != QStringLiteral("audioSpeechOperatorWorkspace")
        || workspace_surface->property("intentWorkspaceActive").toBool()
        || workspace_surface->findChild<QObject*>(QStringLiteral("speechGenerateButton"))
               == nullptr) {
        std::cerr << "desktop graph smoke did not route the packaged speech workspace" << std::endl;
        return false;
    }
    if (!speech_script_help_smoke::verify(root_object))
        return false;
    if (!invoke_packaged_click(
            *workspace_surface,
            QStringLiteral("returnToSceneGraphButton"),
            "desktop graph smoke could not return from speech synthesis"
        )) {
        return false;
    }

    QQmlExpression family_fallback(
        QQmlEngine::contextForObject(host),
        host,
        QStringLiteral(
            "openWorkspace('operator.future.audio', 'operator', 'audio.generate', "
            "'artifact.future', 'revision.future', 'transformation.future')"
        )
    );
    if (!family_fallback.evaluate().toBool() || family_fallback.hasError()) {
        std::cerr << "desktop graph smoke could not open a family fallback" << std::endl;
        return false;
    }
    workspace_surface->setProperty("currentMode", 1);
    QCoreApplication::processEvents();
    if (workspace_surface->property("workspaceRouteKey").toString()
            != QStringLiteral("operator.family.audio")
        || workspace_surface->property("loadedWorkspaceObjectName").toString()
               != QStringLiteral("unknownOperatorWorkspace")) {
        std::cerr << "desktop graph smoke did not route the audio family fallback" << std::endl;
        return false;
    }
    if (!invoke_packaged_click(
            *workspace_surface,
            QStringLiteral("returnToSceneGraphButton"),
            "desktop graph smoke could not return from a family fallback"
        )) {
        return false;
    }

    QQmlExpression unknown_fallback(
        QQmlEngine::contextForObject(host),
        host,
        QStringLiteral(
            "openWorkspace('operator.future.unknown', 'operator', 'future.nebula', "
            "'artifact.future', 'revision.future', 'transformation.future')"
        )
    );
    if (!unknown_fallback.evaluate().toBool() || unknown_fallback.hasError()) {
        std::cerr << "desktop graph smoke could not open an unknown fallback" << std::endl;
        return false;
    }
    workspace_surface->setProperty("currentMode", 1);
    QCoreApplication::processEvents();
    if (workspace_surface->property("workspaceRouteKey").toString()
            != QStringLiteral("operator.unknown")
        || workspace_surface->property("loadedWorkspaceObjectName").toString()
               != QStringLiteral("unknownOperatorWorkspace")) {
        std::cerr << "desktop graph smoke did not route the unknown fallback" << std::endl;
        return false;
    }
    return invoke_packaged_click(
               *workspace_surface,
               QStringLiteral("returnToSceneGraphButton"),
               "desktop graph smoke could not return from an unknown fallback"
           )
           && workspace_surface->property("selectedNodeId").toString()
                  == operator_node->value(QStringLiteral("id")).toString();
}

bool verifyLocalization(QObject& root_object, UiPreferences& ui_preferences) {
    QObject* const workspace_surface =
        root_object.findChild<QObject*>(QStringLiteral("workspaceSurface"));
    QObject* const host =
        workspace_surface == nullptr
            ? nullptr
            : workspace_surface->findChild<QObject*>(QStringLiteral("operatorWorkspaceHost"));
    if (workspace_surface == nullptr || host == nullptr) {
        std::cerr << "desktop localization smoke could not find the packaged Host" << std::endl;
        return false;
    }
    QQmlExpression open_unknown(
        QQmlEngine::contextForObject(host),
        host,
        QStringLiteral(
            "openWorkspace('operator.locale', 'operator', 'future.nebula', "
            "'artifact.locale', 'revision.locale', 'transformation.locale')"
        )
    );
    if (!open_unknown.evaluate().toBool() || open_unknown.hasError()) {
        std::cerr << "desktop localization smoke could not open the unknown fallback" << std::endl;
        return false;
    }
    workspace_surface->setProperty("currentMode", 1);
    QCoreApplication::processEvents();
    QObject* const message = host->findChild<QObject*>(QStringLiteral("unknownWorkspaceMessage"));
    if (message == nullptr) {
        std::cerr << "desktop localization smoke could not find the fallback message" << std::endl;
        return false;
    }

    const QString previous_language = ui_preferences.languageMode();
    ui_preferences.setLanguageMode(QStringLiteral("en"));
    QCoreApplication::processEvents();
    const bool english_ok =
        message->property("text").toString().contains(QStringLiteral("No workspace is registered"));
    ui_preferences.setLanguageMode(QStringLiteral("zh_CN"));
    QCoreApplication::processEvents();
    const bool chinese_ok = message->property("text").toString().contains(QStringLiteral("尚未为"));
    ui_preferences.setLanguageMode(QStringLiteral("en"));
    QCoreApplication::processEvents();
    const bool switched_back =
        message->property("text").toString().contains(QStringLiteral("No workspace is registered"));
    const bool returned =
        QMetaObject::invokeMethod(workspace_surface, "showGraph", Qt::DirectConnection);
    if (!english_ok || !chinese_ok || !switched_back || !returned) {
        std::cerr << "desktop localization smoke did not retranslate the live Host tree"
                  << std::endl;
        return false;
    }

    QQmlExpression open_speech(
        QQmlEngine::contextForObject(host),
        host,
        QStringLiteral(
            "openWorkspace('operator.locale.audio', 'operator', "
            "'audio.speech_synthesize', 'artifact.locale', 'revision.locale', '')"
        )
    );
    if (!open_speech.evaluate().toBool() || open_speech.hasError()) {
        std::cerr << "desktop localization smoke could not open speech synthesis" << std::endl;
        return false;
    }
    workspace_surface->setProperty("currentMode", 1);
    QCoreApplication::processEvents();
    QObject* const speech_title = host->findChild<QObject*>(QStringLiteral("speechWorkspaceTitle"));
    if (speech_title == nullptr) {
        std::cerr << "desktop localization smoke could not find speech title" << std::endl;
        return false;
    }
    ui_preferences.setLanguageMode(QStringLiteral("en"));
    QCoreApplication::processEvents();
    const bool speech_english =
        speech_title->property("text").toString() == QStringLiteral("Voices and audition");
    ui_preferences.setLanguageMode(QStringLiteral("zh_CN"));
    QCoreApplication::processEvents();
    const bool speech_chinese =
        speech_title->property("text").toString() == QStringLiteral("配音与试听");
    ui_preferences.setLanguageMode(QStringLiteral("en"));
    QCoreApplication::processEvents();
    const bool speech_switched_back =
        speech_title->property("text").toString() == QStringLiteral("Voices and audition");
    ui_preferences.setLanguageMode(previous_language);
    QCoreApplication::processEvents();
    return speech_english && speech_chinese && speech_switched_back
           && QMetaObject::invokeMethod(workspace_surface, "showGraph", Qt::DirectConnection);
}

bool verifyProjectWelcome(QObject& root_object) {
    QQuickItem* const welcome =
        root_object.findChild<QQuickItem*>(QStringLiteral("projectWelcome"));
    QQuickItem* const new_project =
        root_object.findChild<QQuickItem*>(QStringLiteral("newProjectButton"));
    QQuickItem* const open_project =
        root_object.findChild<QQuickItem*>(QStringLiteral("openProjectButton"));
    return welcome != nullptr && welcome->isVisible() && welcome->width() > 0.0
           && welcome->height() > 0.0 && new_project != nullptr && new_project->isVisible()
           && new_project->isEnabled() && open_project != nullptr && open_project->isVisible()
           && open_project->isEnabled();
}

bool verifyOperatorDraftRoute(QObject& root_object, DesktopBackend& backend) {
    const auto artifacts = backend.artifacts();
    if (artifacts.size() != 1)
        return false;
    const auto original = artifacts.first().toMap();
    const QString source = original.value(QStringLiteral("id")).toString();
    const QString path = backend.bundlePath();
    auto* surface = root_object.findChild<QObject*>(QStringLiteral("workspaceSurface"));
    auto* host = root_object.findChild<QObject*>(QStringLiteral("operatorWorkspaceHost"));
    if (!surface || !host)
        return false;
    const auto open = [&](const QString& id) {
        QQmlExpression expression(
            QQmlEngine::contextForObject(&root_object),
            &root_object,
            QStringLiteral("openDraftTarget('%1')").arg(id)
        );
        expression.evaluate();
        for (int i = 0; i < 4; ++i)
            QCoreApplication::processEvents();
        return !expression.hasError();
    };
    const QString text_id = backend.beginOperatorDraft(source, QStringLiteral("text.edit"));
    if (text_id.isEmpty() || backend.artifactCount() != 2 || !open(text_id))
        return false;
    auto* writer = qvariant_cast<QObject*>(host->property("loadedWorkspace"));
    if (!writer || writer->objectName() != QStringLiteral("textAuthoringWorkspace")
        || !writer->property("editing").toBool() || !writer->property("inputCurrent").toBool())
        return false;
    auto state = QJsonDocument::fromJson(backend.operatorDrafts()
                                             .first()
                                             .toMap()
                                             .value(QStringLiteral("textAuthoringJson"))
                                             .toString()
                                             .toUtf8())
                     .object();
    state.insert(QStringLiteral("profile"), QStringLiteral("listening"));
    state.insert(QStringLiteral("delivery"), QStringLiteral("clear"));
    state.insert(QStringLiteral("mode"), QStringLiteral("prepare_script"));
    state.insert(
        QStringLiteral("instruction"),
        QStringLiteral("Use the original for a listening exercise.")
    );
    if (!backend.updateTextAuthoring(
            text_id,
            QString::fromUtf8(QJsonDocument(state).toJson(QJsonDocument::Compact))
        ))
        return false;
    if (!backend.openProject(QUrl::fromLocalFile(path)) || !open(text_id))
        return false;
    writer = qvariant_cast<QObject*>(host->property("loadedWorkspace"));
    if (!writer || writer->property("profile").toString() != QStringLiteral("listening")
        || backend.textNodeInput(text_id)
               != original.value(QStringLiteral("textPreview")).toString())
        return false;
    if (!backend.discardOperatorDraft(text_id) || backend.artifactCount() != 1)
        return false;
    const QString speech_id = backend.beginAuthoringSpeech(source);
    if (speech_id.isEmpty() || !open(speech_id))
        return false;
    auto* speech = qvariant_cast<QObject*>(host->property("loadedWorkspace"));
    auto* speed =
        speech ? speech->findChild<QObject*>(QStringLiteral("speechSpeedSlider")) : nullptr;
    if (!speech || !speed || !speech->property("canGenerate").toBool())
        return false;
    const auto speech_draft = backend.operatorDrafts().first().toMap();
    if (!backend.updateAudioSpeechDraft(
            speech_id,
            speech_draft.value(QStringLiteral("audioSpeechPresetAlias")).toString(),
            speech_draft.value(QStringLiteral("audioSpeechPresetCatalogRevision")).toString(),
            QStringLiteral("auto"),
            1150,
            true
        ))
        return false;
    const QString audio_output = backend.operatorDrafts()
                                     .first()
                                     .toMap()
                                     .value(QStringLiteral("contextArtifactId"))
                                     .toString();
    if (!backend.renameArtifact(audio_output, QStringLiteral("Listening recording")))
        return false;
    if (!backend.openProject(QUrl::fromLocalFile(path)) || !open(speech_id))
        return false;
    speech = qvariant_cast<QObject*>(host->property("loadedWorkspace"));
    speed = speech ? speech->findChild<QObject*>(QStringLiteral("speechSpeedSlider")) : nullptr;
    if (!speech
        || speech->property("outputName").toString() != QStringLiteral("Listening recording")
        || !speed || speed->property("value").toInt() != 1150
        || !speech_script_help_smoke::verify(root_object))
        return false;
    if (!backend.discardOperatorDraft(speech_id) || backend.artifactCount() != 1)
        return false;
    root_object.setProperty("selectedArtifactIndex", 0);
    return QMetaObject::invokeMethod(surface, "showGraph", Qt::DirectConnection);
}

} // namespace workspace_host_smoke
