# Infer Runtime candidate.3 Consumer migration

Shape supports the coordinated transition from
`infer-runtime.consumer@0.1.0-candidate.2` to
`infer-runtime.consumer@0.1.0-candidate.3` without changing its App identity,
managed credential, endpoint paths, or local-only security policy.

The upstream contract and complete vocabulary mapping are frozen by Infer
Runtime commit `4f74d54` and its
[candidate.3 migration guide](https://github.com/glenzli/infer-runtime/blob/4f74d54/docs/MIGRATION-0.1.0-candidate.3.md).
This note records only Shape's implemented Consumer boundary.

## Version selection

Shape accepts exactly candidate.2 and candidate.3 during the migration window.
Infra Discovery remains
`infra.discovery.registration@20260810.1` with protocol
`infer-runtime.consumer` and binding `infer-runtime.http-loopback`.

- If an offer contains candidate.3, Shape selects candidate.3.
- Otherwise an offer containing candidate.2 remains eligible temporarily.
- The selected offer version is retained with the endpoint identity.
- The HTTP `/infer/v1/contract` response must match the version selected from
  Discovery. A mismatch fails closed.
- Explicit diagnostic overrides and the temporary fixed-port fallback have no
  advertised version, so Shape uses the probed contract version.
- Candidate.3 additionally requires
  `capability_scale_version="20260811.1"`.

Endpoint owner/mode, no-symlink, canonical numeric-loopback, lease, generation,
proxy-free, redirect-free, and one-retry rediscovery rules do not change.

## Shape request vocabulary

The text generation capability keeps Shape's internal identity `text.generate`.
Only its Runtime wire changes according to the selected contract:

| Wire field | candidate.2 | candidate.3 |
| --- | --- | --- |
| `model` | `assistant.general` | `language.respond` |
| capability metadata key | `infer.quality_floor` | `infer.capability_floor` |
| text capability value | `general` | `foundational` |

The candidate.3 vocabulary migration originally used a mechanical `general` to
`capable` mapping. Shape's interactive text product has since calibrated that
choice against its live local fleet: `capable` admits only the non-interactive
35B deployment, while the validated 4B deployment is rated `foundational`.
Text therefore requests the lower absolute floor explicitly; this is a product
latency/availability choice, not a claim that the two versioned scales are
equivalent.

Speech keeps the Intent `speech.synthesize` and its independently validated
capability floor. Both adapters preserve:

- `infer.policy=local-first`;
- `infer.placement=local_only`;
- `infer.prefer=local`;
- `infer.offline_required=true`;
- `infer.fallback=none`;
- `infer.max_cost_usd=0`.

Shape never sends both vocabularies in one request and never treats candidate.2
`advanced` as candidate.3 `advanced`.

## Provenance

New Shape provenance uses:

- `capability_level` instead of `quality_grade`;
- `evaluation_status` instead of `rating_status`;
- `capability_floor` instead of `quality_floor`.

Runtime Job snapshots are decoded strictly according to their contract version.
Candidate.2 snapshots remain readable during migration, but candidate.2 fields
are rejected when a candidate.3 contract was negotiated and vice versa.
Previously persisted Shape provenance remains readable through deserialize-only
aliases; newly serialized provenance uses candidate.3 field names.

## Rollout gate

Shape is ready to select candidate.3 when the Runtime publisher exposes it.
Before removing candidate.2 support or the fixed endpoint fallback:

1. restart the Runtime with the candidate.3 build and observe a new Discovery
   generation;
2. run one non-sensitive `language.respond` smoke using the existing Shape
   credential;
3. verify `capability_level`, `evaluation_status`,
   `routing.capability_floor`, and the unchanged local-only/no-fallback policy;
4. complete the coordinated Consumer soak across all registered tenants.

This migration never requires reading, rotating, or reinstalling the Shape
credential.
