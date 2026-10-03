#!/usr/bin/env python3
"""Bounded real Echo fixture driver; no GUI, user project, credential or network access."""
import argparse
import hashlib
import json
import os
from pathlib import Path
import selectors
import subprocess


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--binary", required=True, type=Path)
    parser.add_argument("--source", required=True, type=Path)
    parser.add_argument("--evidence", required=True, type=Path)
    args = parser.parse_args()
    args.evidence.mkdir(exist_ok=False)
    env = dict(os.environ, ECHO_DEBUG_EDITOR_SESSION_HOME=str(args.evidence / "sessions"),
               QT_QPA_PLATFORM="offscreen", QSG_RHI_BACKEND="software",
               RAYON_NUM_THREADS="1", OMP_NUM_THREADS="1")
    log = (args.evidence / "stderr.log").open("wb")
    proc = subprocess.Popen([str(args.binary), "--editor-stdio", "--input", str(args.source)],
                            stdin=subprocess.PIPE, stdout=subprocess.PIPE, stderr=log,
                            env=env, bufsize=0)
    selector = selectors.DefaultSelector()
    selector.register(proc.stdout, selectors.EVENT_READ)
    transcript = []

    def read():
        # Per-message deadline and size bound, including a child that never emits a newline.
        import time
        deadline = time.monotonic() + 20
        raw = bytearray()
        while True:
            remaining = deadline - time.monotonic()
            if remaining <= 0 or not selector.select(remaining):
                raise TimeoutError("Echo response deadline exceeded")
            byte = os.read(proc.stdout.fileno(), 1)
            if not byte:
                raise RuntimeError("Echo closed its protocol stream")
            if byte == b"\n":
                value = json.loads(raw)
                transcript.append(value)
                return value
            raw.extend(byte)
            if len(raw) > 1024 * 1024:
                raise ValueError("Echo response exceeds one MiB")

    def send(request):
        proc.stdin.write(json.dumps(request).encode() + b"\n")
        proc.stdin.flush()
        return read()

    try:
        ready = read()
        assert ready["ok"] and ready["command"] == "ready", ready
        base = ready["snapshot"]["base"]
        receipts = []
        for index, gain in enumerate((-600, -1200)):
            applied = send({"command": "apply", "request_key": f"shape_edit_{index}",
                            "base": base, "gain_centibels": gain})
            assert applied["ok"], applied
            base = applied["base"]
            request = {"command": "export", "request_key": f"shape_return_{index}",
                       "base": base, "destination": str(args.evidence / f"return-{index}.wav"),
                       "timeout_ms": 10000}
            receipt = send(request)
            assert receipt["ok"] and receipt["status"] == "published", receipt
            assert receipt["audio"]["bit_depth"] == 24, receipt
            replay = send(request)
            assert replay["ok"] and replay["status"] == "published", replay
            assert replay["output_hash_blake3"] == receipt["output_hash_blake3"]
            receipts.append(receipt)
        report = {"binary": str(args.binary),
                  "binary_sha256": hashlib.sha256(args.binary.read_bytes()).hexdigest(),
                  "ready": ready, "receipts": receipts, "transcript": transcript}
        (args.evidence / "report.json").write_text(json.dumps(report, indent=2) + "\n")
    finally:
        proc.stdin.close()
        try:
            proc.wait(timeout=5)
        except subprocess.TimeoutExpired:
            proc.terminate()
            try:
                proc.wait(timeout=3)
            except subprocess.TimeoutExpired:
                proc.kill()
                proc.wait(timeout=3)
        selector.close()
        log.close()


if __name__ == "__main__":
    main()
