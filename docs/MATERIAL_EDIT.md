# Material editing stages

`image.external_edit` (Shadow) and `audio.external_edit` (Echo) are persistent
working-graph stages. A stage reserves its own output Artifact, pins one accepted
source revision, and owns a stable producer node. The source is never replaced.
An external editor's successful export creates reviewable material. Only Shape's
explicit **Adopt** / CLI `accept` creates an accepted revision on that output.

## Human workflow

1. Select an accepted image or audio clip and add **Edit in Shadow** or **Edit in Echo**.
2. For images, choose **Open in Shadow**. Shape exports a fresh temporary working
   directory, launches Shadow with `--pipeline-edit --request ... --result ...`,
   and waits for its native completion. **Choose Shadow…** stores an explicit
   application/executable selection in local desktop settings. An unavailable
   explicit selection does not silently fall back to another editor. Without a
   selection, discovery checks `SHAPE_SHADOW_BINARY`, `PATH`, and macOS
   `/Applications/Shadow.app`. Executability is discovery only; the actual strict
   protocol determines compatibility. Use Shadow's **Return edited image** to
   return a candidate automatically, then review and explicitly adopt it.
   The manual fallback exports into a selected external folder. Shape creates a new
   subfolder with `source.png` / `source.wav` and publishes `handoff.json` last.
3. Edit that copy in the editor, save/export a different file, then select that
   returned file and its original handoff in Shape. Review the image against its
   pinned original or audition the original/returned audio.
4. Adopt the reviewed result. Downstream stages continue to reference their prior
   source revision until their input is explicitly refreshed.

Echo's current native exporter returns 48 kHz stereo PCM24 WAV. Shape's accepted
audio contract is PCM16. **Convert Echo PCM24 returns to PCM16 for Shape** is off
by default and must be explicitly enabled for those returns. This is a precision
reduction: nearest rounding, ties away from zero, saturated to signed 16 bit;
no resampling, remix, gain adjustment or dither occurs in Shape. The raw returned
BLAKE3 and `pcm24_to_pcm16` operation are recorded. The external PCM24 file remains
intact. RIFF ancillary chunk payloads, including Echo disclosure metadata, are
retained; the accepted audio origin also inherits the Shape source's disclosure.
The converter supports bounded mono/stereo plain PCM24 and PCM24 extensible WAV
with PCM GUID and supported speaker masks. Other formats are rejected.

The manual file handoff does not launch a provider, attach to a live GUI, infer unsaved
changes, or claim that the named editor ran. A human may edit the working copy;
an AI may use the provider's native tools. Shape's receipt is explicitly a local
return verifier, never a fabricated provider execution receipt. The current
Shadow version must support the prepared PNG input. The only ICC preparation is
the exact known-sRGB working-copy operation described below; do not transcode to
JPEG, discard alpha or strip an arbitrary profile to bypass a provider limitation.

## AI interface

Run `shape-cli material-edit /absolute/project.shape /absolute/request.json`.
Requests are strict JSON; unknown fields fail. Stdout is one JSON object with
`schema: "shape.material-edit-result/1"` and `status: "ok" | "error"`.
Errors also return a nonzero process status. No credentials or editor paths are
embedded in accepted history.

| Action | Request fields beyond `action` | Result |
| --- | --- | --- |
| `read` | Exact `source: {artifact_id, revision_id}` | Verified accepted revision, media contract and material-return lineage; no writes |
| `begin` | `source: {artifact_id, revision_id}`, `name`, `editor: "shadow" | "echo"` | Opaque `ticket` |
| `status` | `output_artifact_id` | Current ticket after reopening |
| `export` | `ticket`, `destination` (new external directory) | Source copy and `handoff` |
| `review` | `handoff` (JSON path), `returned_file`, optional `allow_pcm24_conversion` | Raw/normalized digests, media contract, normalization, `accepted:false` |
| `accept` | Same paths/policy plus exact reviewed `returned_blake3` | Accepted revision |
| `review_shadow` | `handoff`, `result_file` | Completed native Shadow result, independently verified as a candidate |
| `accept_shadow` | Same paths plus reviewed `returned_blake3` | Revalidate receipt/bytes and explicitly accept |
| `cancel` | `ticket` | Session advances; previous returns invalidated |
| `refresh` | `ticket` | Explicitly binds current source revision and advances session |

Treat the ticket as opaque. Preserve it unchanged across tools. `accept` rereads
and hashes the file, so a modification after review cannot be accepted under the
old digest. An AI must carry the reviewed conversion policy into `accept` as well.
Candidate review is transient; reopening reconstructs a candidate from the saved
handoff and selected return. Accepted revisions and stage identity survive reopening.

### Resume from an accepted material version

`project-status` supplies accepted Artifact/revision identities. To recover the content reference
and media contract after reopening, pass that exact pin to `read`:

```json
{"action":"read","source":{"artifact_id":"<artifact UUID>","revision_id":"<revision UUID>"}}
```

The result includes `source`, `artifact`, full immutable `revision` (content BLAKE3, byte length,
media type, contract, parents and transformation ID), and `material_edit`. The latter is null for
an ordinary imported/generated material; an accepted external return includes its pinned source,
input/returned digests, editor, stage/session and explicit normalization. These are stored Shape
lineage, not proof that a named provider ran. `content_verified:true` means the durable object's
exact bytes matched the stored length and BLAKE3 during this read. Payload bytes and external paths
are not returned. ContentRef and lineage digests retain the Domain JSON representation (arrays of
32 bytes); copy `revision.content` unchanged when supplying a cue binding. The separate
`returned_blake3` review/accept field remains hexadecimal. `accepted:true` refers to that durable revision, which can be historical.

A newer head never substitutes for the requested revision. `artifact.accepted_revision` is the
head observed with the revision metadata in one SQLite read transaction; it is not a lease for
later adoption. Wrong artifact/revision pairs, non-image/audio media and missing or corrupt bytes
fail without partial output. This action uses the current-schema read-only Store opener, never
schema migration or repair. Its limits are 64 KiB manifest, 4 MiB inspected metadata and 128 MiB
content, plus a 250 ms SQLite busy timeout. SQLite may use its ordinary WAL locking sidecars.

A book caller can read the old composition with `picture-book read`, explicitly replace a page's
source with the recovered pin, then preview and accept the new composition. Repeated pages and
old book versions keep their prior pins. An audio caller can use `revision.content` in an explicit
`SpeechCueAction::Audio` binding passed to `script-check --bindings`; cue limits remain 8 MiB each
and 32 MiB combined. That command checks portable script/choice readiness, not project cue
persistence, WAV playback, Runtime availability or synthesis. It continues to report
`synthesis_checked:false`. Neither consumer auto-follows a newer material head or auto-accepts it.

## Concurrency, cancellation and recovery

Acceptance checks project identity, exact source bytes/head, stage configuration,
session, output head and producer identity. Store repeats lineage and media checks
and compares heads/node state in the durable transaction. A human or second AI
advancing any of those bindings makes an old return stale. Refresh/cancel uses the
observed graph CAS and does not replace another session's draft.

Repeated identical review keeps one candidate. Invalid replacement leaves an
already reviewed candidate intact. Repeated acceptance cannot append the same
stale return. Cancellation invalidates both pending worker results and reviewed
candidates, retaining the stage, original, accepted output and exported files.
Background completion for stage A cannot navigate, populate or show errors in B.
The native launcher additionally compares the complete ticket before launch and
before reading a completion. Its own `QProcess` observes the launched editor's
exit: no completed result stops waiting with an incomplete-return error and a
retry path. A known startup failure or exit without a receipt clears that ticket's
local recovery record, so reopening cannot resume a known failed wait. There is
no short timeout on normal human editing. Cancellation invalidates the watcher
and leaves Shadow's independently running window and exported files intact.
Closing or crashing Shape never turns a later completion into accepted content
and does not cancel that stage.

The desktop stores the automatic handoff/result paths and complete ticket in
local settings, scoped by canonical bundle location and output stage, with project
identity checked in the record. Copies of a project can retain identical portable
identities without overwriting one another's local return location. Earlier local
records migrate only after their bundle path and complete ticket match; cancelling
one bundle cannot clear another copy's continuation. Reopening the still-current
stage restores waiting or reconstructs a reviewable candidate from its existing
receipt. It verifies project/path/ticket identity before waiting, then the core
rechecks the handoff, source and native receipt. These machine paths are not in
accepted history. The file controls also expose the working-copy location, and
manual `handoff.json` / `shadow-result.json` selection remains available.
Temporary launch folders may be removed by the operating system; use a manually
exported folder for retained working copies. Process launch, exit and recovery
never imply successful production or acceptance.

File reads require regular non-symlink files, bounded sizes and exact bytes. WAV
returns are limited to 128 MiB; the PNG decoder additionally enforces its raster
limits. Request JSON is at most 256 KiB, a handoff 128 KiB. Exports are no-clobber;
if writing fails, a partial external directory may remain. Only a complete
`handoff.json` is usable. It is safe to export into a new directory and retry.

Echo stdio tools retain their native base/request-key contract. Require
`ok:true`, `status:"published"`, verified output bytes/digest and the expected
session before selecting a return. `published_unrecorded` or
`prepared_unresolved` is not success: resume the same private session and replay
the exact original request. Saved human `.echo` input requires Echo's exact
project-hash intake, not a live GUI draft. These provider facts are outside the
Shape ticket and never grant authority to mutate its project store.

## Verification

Rust tests cover source/output CAS, cancellation, stale downstream refresh,
foreign tickets, invalid/repeated return, media/disclosure contracts, file bounds
and CLI behavior. The resumed CLI consumer tests separately cover exact historical material
reads, wrong identity/corrupt bytes, bilingual book repinning with preserved older compositions,
and an explicitly converted PCM24 return reused as a portable speech cue. Store tests verify
unchanged manifest/database/WAL bytes, refusal to migrate older schemas and bounded metadata
before deserialization. These synthetic-return tests make no actual Echo/Shadow execution claim.
`material_edit_smoke` exercises the real packaged workspace and
asynchronous completion with synthetic pixels/WAV, actual audio review controls,
explicit precision conversion, audition decoding and media-correct navigation labels. The opt-in public API test runs
an actual separately built Echo binary in an isolated private session:

```sh
SHAPE_ECHO_BINARY=/absolute/Echo \
SHAPE_ECHO_EVIDENCE=/absolute/new-evidence-directory \
CARGO_BUILD_JOBS=1 cargo test --offline -p shape-core --test material_edit_echo \
  -- --ignored --test-threads=1
```

This test does not open a visible UI, spend on inference, or change real user
materials. GUI-human return in Echo requires its own provider integration
verification; this slice does not add a new Echo GUI claim. Real Shadow producer
and desktop launcher checks are described separately below; do not infer them
from synthetic returns.


## Native Shadow completion

Image handoff export also writes `shadow-request.json` beside `handoff.json`.
Launch the verified Shadow build with `--pipeline-edit --request <that file>
--result <handoff-directory>/shadow-result.json`, optionally with `--agent-stdio`.
The output path is reserved as `<handoff-directory>/edited.png`. The human Return
edited image control and stdio complete action share the producer's export owner.

Choose `shadow-result.json` in the desktop return picker, or use CLI
`review_shadow` / `accept_shadow`. Shape requires completed status, empty errors,
exact request ID/input revision/path/SHA-256, the reserved output path, actual
output SHA-256/byte length, sRGB/8-bit declarations and decoded dimensions.
Failed/cancelled/incomplete results are rejected even when an output file exists.
The final accept still performs Shape's own source/stage/output CAS. Normal
manual PNG review remains a local verifier and makes no provider execution claim.
The provider JSON stays in the external handoff; accepted history records Shape
lineage and the raw returned BLAKE3, not a signed provider attestation.

The fixed Shadow producer supports only RGB8/fully opaque RGBA8 PNG, untagged
sRGB or an sRGB chunk, at most 64 MiB, 16,777,216 pixels and 16,384 pixels per
axis. Transparency, 16-bit and arbitrary ICC input remain unsupported. Its
returned PNG contains Qt's standard sRGB profile. Shape recognizes only that
complete 480-byte profile, with BLAKE3
`fc9a5af37a9933652f8992e652b8a4cd303eb58318240f1d97dff993a34e567a`
(SHA-256 `ef087eece0b3a8ca7b79e56b8e82578bea636870a4bd547c44c25722605ff838`).
`shape-execution::raster::shadow_input` pins those bytes; the verified producer
uses `QColorSpace::SRgb` in Shadow's `backend/export_backend.cpp`. Descriptions,
declared primaries or profile names alone never authorize preparation.

For this exact profile on opaque 8-bit PNG, handoff export preserves `source.png`
byte-for-byte and writes the same decoded RGBA samples to `shadow-source.png`
without an ICC chunk. `handoff.json` records `known_srgb_icc_input`, the profile
BLAKE3 and prepared-byte BLAKE3; the native request pins the prepared path and
SHA-256. Shape recomputes that preparation when reviewing and accepting the
return, and Store independently verifies it from the original CAS source before
durable acceptance. The accepted transformation records the same explicit
normalization. This is an identity on known sRGB samples, not general ICC color
conversion. Unknown or even one-byte-mutated profiles fail closed. The launcher
explains this preparation beside **Open in Shadow**, without an extra dialog.
Other consumers may use the preserved accepted PNG directly. Export cancellation
inside Shadow remains unsupported; Shape cancellation invalidates acceptance
and ignores late completion without promising to interrupt the renderer.

The opt-in `material_edit_shadow` test uses `SHAPE_SHADOW_BINARY` and a new
`SHAPE_SHADOW_EVIDENCE` directory. It runs actual public stdio/external-stage
processes offscreen, accepts two separate versions, checks pinned downstream
refresh and verifies original bytes and reopened history. It then uses an actual
accepted ICC-tagged return as the source of a new Shadow stage, verifies the
prepared copy and rejection of tampering, and runs the real producer again before
accepting/reopening the second-stage lineage. This repeated-edit path is
additional to, not replaced by, the producer's own tests.

The desktop picture-book smoke has an independent opt-in real launcher branch
when `SHAPE_SHADOW_BINARY` names that verified binary. It exercises Shadow's human
return control, cancellation with a late receipt, and destruction/recreation of
the Shape controller while Shadow continues. A short-lived executable separately
checks that exit without a receipt releases the busy state. These checks create
isolated projects, local settings and Shadow profiles; they do not edit user
materials or prove unrestricted compatibility with other Shadow builds.

A separate two-process branch of `--smoke-picture-book` is explicitly enabled by
`SHAPE_BOOK_PROCESS_RESTART_PHASE=launch` and then `resume`, with the same new
absolute fixture directory and `SHAPE_SHADOW_BINARY`. The launch phase saves an
unfinished book checkpoint and local handoff, then lets the complete Shape process
exit while the real Shadow result is pending. An outer runner waits for that PID
to exit and Shadow to complete before starting a fresh Shape PID for recovery.
The resume phase requires the exact original bytes, unfinished script and stage,
reconstructs only a candidate, then explicitly cancels it. This branch does not run
in the ordinary offline smoke and never reads the user's inference credentials.

The native Shadow request ID hashes the complete Shape ticket, including the expected
output head. Rebinding an earlier completed receipt to a later output version is
rejected even when the stage and source revision remain the same.
