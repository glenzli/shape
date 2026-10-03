#!/usr/bin/env python3
"""Exercise the real CMake sync against Cargo's machine-readable build output."""
import json
from pathlib import Path
import stat
import subprocess
import tempfile
import sys

script = Path(__file__).resolve().parents[1] / "sync_cxxbridge_headers.cmake"
with tempfile.TemporaryDirectory(prefix="shape-cxx-sync-") as directory:
    root = Path(directory)
    headers = ("shape-desktop-bridge/src/lib.rs.h", "rust/cxx.h")
    for generation in ("old", "active"):
        for name in headers:
            file = root / "debug/build" / f"shape-desktop-bridge-{generation}" / "out/cxxbridge/include" / name
            file.parent.mkdir(parents=True, exist_ok=True)
            file.write_text(generation)
    active = root / "debug/build/shape-desktop-bridge-active/out"
    records = [{"reason":"build-script-executed", "package_id":"path+file:///fixture/shape-desktop-bridge#0.1.0", "out_dir":str(active)}]
    manifest = root / "cargo.json"
    manifest.write_text(json.dumps(records))
    fake = root / "cargo-fixture"
    fake.write_text(f"#!{sys.executable}\nimport json\nfrom pathlib import Path\nfor record in json.loads(Path({str(manifest)!r}).read_text()): print(json.dumps(record))\n")
    fake.chmod(fake.stat().st_mode | stat.S_IXUSR)
    stable = root / "cxxbridge"
    for name in headers:
        path = stable / name; path.parent.mkdir(parents=True, exist_ok=True)
        path.symlink_to(root / "debug/build/shape-desktop-bridge-old/out/cxxbridge/include" / name)
    command = ["cmake", f"-DSHAPE_CARGO_EXECUTABLE={fake}", f"-DSHAPE_SOURCE_DIRECTORY={root}",
               f"-DSHAPE_CARGO_TARGET_DIRECTORY={root}", "-DSHAPE_CARGO_PROFILE=debug",
               f"-DSHAPE_CXXBRIDGE_INCLUDE_DIRECTORY={stable}", "-P", str(script)]
    result = subprocess.run(command, capture_output=True, text=True, timeout=20)
    assert result.returncode == 0, result.stderr
    for name in headers:
        assert (stable / name).read_text() == "active" and not (stable / name).is_symlink()
        assert (root / "debug/build/shape-desktop-bridge-old/out/cxxbridge/include" / name).read_text() == "old"
    records.append(dict(records[0], out_dir=str(root / "debug/build/shape-desktop-bridge-old/out")))
    manifest.write_text(json.dumps(records))
    result = subprocess.run(command, capture_output=True, text=True, timeout=20)
    assert result.returncode != 0 and "found 2" in result.stderr
    assert all((stable / name).read_text() == "active" for name in headers)
    manifest.write_text("[]")
    result = subprocess.run(command, capture_output=True, text=True, timeout=20)
    assert result.returncode != 0 and "found 0" in result.stderr
    print("PASS active output selected; old cache/symlink preserved; ambiguous and missing authority rejected")
