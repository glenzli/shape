.pragma library

// Read-only work relationships derived from the persisted Artifact input bindings.
// This is a display projection, not a Scene Working Graph or an editable port model.

function indexArtifacts(artifacts) {
    const byId = {}
    for (const artifact of artifacts || []) byId[String(artifact.id)] = artifact
    return byId
}

function inputReference(artifact, byId) {
    if (!artifact || (artifact.kindKey !== "text_document" && artifact.kindKey !== "audio_clip"))
        return null
    const nodes = artifact.operatorNodes || []
    const edges = artifact.operatorEdges || []
    const expected = artifact.kindKey === "audio_clip"
                     ? "audio.speech_synthesize" : "text.edit"
    for (const node of nodes) {
        if (node.roleKey !== "source" || !node.artifactId
                || String(node.artifactId) === String(artifact.id)
                || !byId[String(node.artifactId)]
                || byId[String(node.artifactId)].kindKey !== "text_document") continue
        if (edges.some(edge => edge.sourceNodeId === node.id
                       && nodes.some(target => target.id === edge.targetNodeId
                                     && target.roleKey === "operator"
                                     && (target.operatorTypeKey === expected
                                         || (artifact.kindKey === "text_document"
                                             && target.operatorTypeKey === "text.transform"))))) {
            return {parentId: String(node.artifactId), sourceNode: node}
        }
    }
    return null
}

function rootId(artifactId, byId) {
    let current = String(artifactId)
    const visited = {}
    while (byId[current] && !visited[current]) {
        visited[current] = true
        const reference = inputReference(byId[current], byId)
        if (!reference || visited[reference.parentId]) break
        current = reference.parentId
    }
    return current
}

function groupWorks(artifacts) {
    const byId = indexArtifacts(artifacts)
    const groups = []
    const groupsById = {}
    for (const artifact of artifacts || []) {
        const id = String(artifact.id)
        if (rootId(id, byId) !== id) continue
        const group = {id: id, name: artifact.name, kindKey: artifact.kindKey,
                       kindLabel: artifact.kindLabel, outputs: []}
        groups.push(group)
        groupsById[id] = group
    }
    for (const artifact of artifacts || []) {
        const id = String(artifact.id)
        const root = rootId(id, byId)
        if (root !== id && groupsById[root]) groupsById[root].outputs.push(artifact)
    }
    return groups
}

function graphFor(artifacts, selectedId, drafts) {
    const byId = indexArtifacts(artifacts)
    if (!byId[String(selectedId)]) return {rootId: "", nodes: [], edges: [], artifactIds: []}
    const root = rootId(selectedId, byId)
    const included = (artifacts || []).filter(artifact => rootId(artifact.id, byId) === root)
    included.sort((a, b) => {
        function depth(artifact) {
            let count = 0
            let current = artifact
            while (current && String(current.id) !== root && count < included.length) {
                const reference = inputReference(current, byId)
                current = reference ? byId[reference.parentId] : null
                ++count
            }
            return count
        }
        return depth(a) - depth(b)
    })

    const nodes = []
    const edges = []
    const knownIds = {}
    for (const artifact of included) {
        const reference = inputReference(artifact, byId)
        const parent = reference ? byId[reference.parentId] : null
        const parentOutputId = parent ? "output." + parent.id : ""
        const canJoin = parent && knownIds[parentOutputId] !== undefined
        const usesEarlierVersion = canJoin && reference.sourceNode.revisionId
                                   && String(reference.sourceNode.revisionId)
                                      !== String(parent.acceptedRevisionId)
        const localIds = {}
        const artifactNodes = artifact.operatorNodes || []
        const artifactEdges = artifact.operatorEdges || []
        const importSource = String(artifact.id) === root
                             && artifact.kindKey === "text_document"
                             && artifactNodes.length === 2 && artifactEdges.length === 1
                             && artifactNodes.some(node => node.roleKey === "source")
                             && artifactNodes.some(node => node.roleKey === "output")
        const authoringDraft = (drafts || []).find(draft =>
            String(draft.contextArtifactId) === String(artifact.id)
            && draft.operatorTypeKey === "text.create")
        let manualCreation = false
        if (String(artifact.id) === root && artifact.kindKey === "text_document"
                && artifactNodes.length === 2 && artifactEdges.length === 1
                && authoringDraft && authoringDraft.textAuthoringJson) {
            try {
                manualCreation = JSON.parse(authoringDraft.textAuthoringJson).entry === "manual"
            } catch (_) { /* An older or invalid draft must not change the saved graph. */ }
        }
        const rootSource = importSource
                           ? artifactNodes.find(node => node.roleKey === "source")
                           : manualCreation
                             ? artifactNodes.find(node => node.roleKey === "operator"
                                 && node.operatorTypeKey === "text.create") : null
        for (const node of artifactNodes) {
            if (rootSource && node.id === rootSource.id) continue
            if (canJoin && node.id === reference.sourceNode.id) continue
            let id = String(node.id)
            if (knownIds[id] !== undefined) id += ".in." + artifact.id
            localIds[String(node.id)] = id
            let copy = Object.assign({}, node, {id: id})
            if (rootSource && node.roleKey === "output") {
                copy = Object.assign({}, copy, {
                    roleKey: "source", outputPorts: rootSource.outputPorts
                })
                if (importSource) {
                    copy.hasTextPreview = rootSource.hasTextPreview
                    copy.textPreview = rootSource.textPreview
                    copy.textPreviewTruncated = rootSource.textPreviewTruncated
                }
            }
            nodes.push(copy)
            knownIds[id] = nodes.length - 1
        }
        if (canJoin) {
            const output = nodes[knownIds[parentOutputId]]
            const dataType = reference.sourceNode.outputPorts[0].dataTypeKey
            if ((output.outputPorts || []).length === 0) {
                nodes[knownIds[parentOutputId]] = Object.assign({}, output, {
                    outputPorts: [{id: "output.value", dataTypeKey: dataType}]
                })
            }
        }
        for (const edge of artifactEdges) {
            if (rootSource && edge.sourceNodeId === rootSource.id) continue
            const fromReference = canJoin && edge.sourceNodeId === reference.sourceNode.id
            edges.push(Object.assign({}, edge, {
                sourceNodeId: fromReference ? parentOutputId
                                           : localIds[String(edge.sourceNodeId)],
                targetNodeId: localIds[String(edge.targetNodeId)],
                earlierInput: fromReference && usesEarlierVersion,
                pinnedRevisionId: fromReference ? reference.sourceNode.revisionId : ""
            }))
        }
    }
    return {rootId: root, nodes: nodes, edges: edges,
            artifactIds: included.map(artifact => String(artifact.id))}
}
