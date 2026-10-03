# Material editing stages

`image.external_edit` (Shadow) and `audio.external_edit` (Echo) are persistent
working-graph stages. A stage reserves its own output Artifact, pins one accepted
source revision, and owns a stable producer node. The source is never replaced.
An external editor's successful export creates reviewable material. Only Shape's
explicit **Adopt** / CLI `accept` creates an accepted revision on that output.

## Human workflow

1. Select an accepted image or audio clip and add **Edit in Shadow** or **Edit in Echo**.
2. Export a working copy into a selected external folder. Shape creates a new
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

The file handoff does not launch a provider, attach to a live GUI, infer unsaved
changes, or claim that the named editor ran. A human may edit the working copy;
an AI may use the provider's native tools. Shape's receipt is explicitly a local
return verifier, never a fabricated provider execution receipt. The current
Shadow version must support the canonical PNG input; do not silently transcode
to JPEG or discard alpha/ICC to bypass a provider limitation.

## AI interface

Run `shape-cli material-edit /absolute/project.shape /absolute/request.json`.
Requests are strict JSON; unknown fields fail. Stdout is one JSON object with
`schema: "shape.material-edit-result/1"` and `status: "ok" | "error"`.
Errors also return a nonzero process status. No credentials or editor paths are
embedded in accepted history.

| Action | Request fields beyond `action` | Result |
| --- | --- | --- |
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
and CLI behavior. `material_edit_smoke` exercises the real packaged workspace and
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
materials. GUI-human return in Echo and live Shadow editing require their own
provider integration verification; do not infer those from synthetic return tests.


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
sRGB or an sRGB chunk, at most 64 MiB and 16,777,216 pixels. Transparency, 16-bit
and arbitrary ICC input remain unsupported. Its returned PNG contains a standard
sRGB ICC profile. Re-entering that result into another Shadow stage requires an
explicit known-sRGB materialization; Shape does not silently strip the profile.
Other consumers may use the preserved accepted PNG directly. Export cancellation
inside Shadow remains unsupported; Shape cancellation invalidates acceptance
and ignores late completion without promising to interrupt the renderer.

The opt-in `material_edit_shadow` test uses `SHAPE_SHADOW_BINARY` and a new
`SHAPE_SHADOW_EVIDENCE` directory. It runs actual public stdio/external-stage
processes offscreen, accepts two separate versions, checks pinned downstream
refresh and verifies original bytes and reopened history. It is additional to,
not replaced by, the producer's own tests.

The native Shadow request ID hashes the complete Shape ticket, including the expected
output head. Rebinding an earlier completed receipt to a later output version is
rejected even when the stage and source revision remain the same.
