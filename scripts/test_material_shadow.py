#!/usr/bin/env python3
"""One bounded Shadow external stage, through its actual public tool protocol."""
import argparse
import hashlib
import json
import os
from pathlib import Path
import selectors
import subprocess
import time


def main():
    p = argparse.ArgumentParser()
    p.add_argument("--binary", required=True, type=Path)
    p.add_argument("--request", required=True, type=Path)
    p.add_argument("--result", required=True, type=Path)
    p.add_argument("--evidence", required=True, type=Path)
    p.add_argument("--exposure", required=True, type=float)
    args = p.parse_args()
    args.evidence.mkdir(exist_ok=False)
    envelope = json.loads(args.request.read_text())
    before_binary = hashlib.sha256(args.binary.read_bytes()).hexdigest()
    env = dict(os.environ, QT_QPA_PLATFORM="offscreen", QT_QUICK_BACKEND="software",
               SHADOW_DESKTOP_DATA_ROOT=str(args.evidence / "private-profile"),
               SHADOW_PAINT_BRUSH_SETTINGS_FILE=str(args.evidence / "brush-settings.ini"),
               RAYON_NUM_THREADS="1", OMP_NUM_THREADS="1")
    log = (args.evidence / "stderr.log").open("wb")
    process = subprocess.Popen([str(args.binary), "--pipeline-edit", "--request", str(args.request),
                                "--result", str(args.result), "--agent-stdio"], env=env,
                               stdin=subprocess.PIPE, stdout=subprocess.PIPE, stderr=log, bufsize=0)
    selector = selectors.DefaultSelector()
    selector.register(process.stdout, selectors.EVENT_READ)
    transcript, buffer = [], bytearray()
    sequence = 0

    def call(op, params=None):
        nonlocal sequence, buffer
        sequence += 1
        request = {"schema": "shadow.edit-tools/1", "id": str(sequence), "op": op, "params": params or {}}
        process.stdin.write(json.dumps(request).encode() + b"\n")
        process.stdin.flush()
        deadline = time.monotonic() + 25
        while True:
            while b"\n" in buffer:
                line, _, remainder = buffer.partition(b"\n")
                buffer = bytearray(remainder)
                response = json.loads(line)
                assert response["schema"] == request["schema"] and response["id"] == request["id"], response
                transcript.append({"request": request, "response": response})
                return response
            remaining = deadline - time.monotonic()
            if remaining <= 0 or not selector.select(remaining):
                raise TimeoutError(f"Shadow {op} response deadline")
            data = os.read(process.stdout.fileno(), 65536)
            assert data, "unexpected Shadow protocol EOF"
            buffer.extend(data)
            assert len(buffer) <= 512 * 1024, "response exceeds bound"

    def checked(response):
        assert response["ok"], response
        return response["result"]

    def snapshot():
        deadline = time.monotonic() + 40
        while time.monotonic() < deadline:
            response = call("snapshot")
            if response["ok"]:
                state = response["result"]
                if not any(state[key] for key in ("busy", "dirty", "autosavePending", "gestureActive")):
                    return state
            else:
                assert response["error"]["code"] in ("busy", "not_ready"), response
            time.sleep(0.05)
        raise TimeoutError("Shadow did not become idle")

    try:
        caps = checked(call("discover"))
        assert caps["externalStage"]["input"] == envelope["input"]
        initial = snapshot()
        node = next(n for n in initial["nodes"] if n["editable"])
        params = {"expected": initial["identity"], "operations": [
            {"type": "set_exposure", "nodeId": node["nodeId"], "stops": args.exposure}],
            "outputPath": str(args.evidence / "preview.jpg")}
        deadline = time.monotonic() + 30
        while True:
            response = call("preview", params)
            if response["ok"]:
                proposal = response["result"]
                break
            assert response["error"]["code"] == "busy" and time.monotonic() < deadline, response
            params["expected"] = snapshot()["identity"]
        reviewed = snapshot()
        checked(call("apply", {"expected": reviewed["identity"], "proposalId": proposal["proposalId"]}))
        final = snapshot()
        checked(call("complete", {"expected": final["identity"]}))
        assert process.wait(timeout=60) == 0
        receipt = json.loads(args.result.read_text())
        assert receipt["outcome"] == "completed" and receipt["input"] == envelope["input"]
        assert receipt["requestId"] == envelope["requestId"]
        raw = Path(receipt["artifact"]["path"]).read_bytes()
        assert hashlib.sha256(raw).hexdigest() == receipt["artifact"]["sha256"]
        assert receipt["artifact"]["colorSpace"] == "sRGB" and receipt["artifact"]["bitDepth"] == 8
        assert hashlib.sha256(Path(envelope["input"]["path"]).read_bytes()).hexdigest() == envelope["input"]["sha256"]
        assert hashlib.sha256(args.binary.read_bytes()).hexdigest() == before_binary
        (args.evidence / "report.json").write_text(json.dumps({"binary":str(args.binary),
            "binary_sha256":before_binary,"receipt":receipt,"transcript":transcript,
            "source_unchanged":True,"offscreen":True},indent=2)+"\n")
    finally:
        if process.poll() is None:
            process.terminate()
            try:
                process.wait(timeout=5)
            except subprocess.TimeoutExpired:
                process.kill(); process.wait(timeout=5)
        selector.close(); process.stdin.close(); process.stdout.close(); log.close()


if __name__ == "__main__":
    main()
