"""Compile the actual encounter.cpp against a small rendering API fixture on MSVC."""
import pathlib
import shutil
import subprocess
import tempfile
import sys

root = pathlib.Path(__file__).resolve().parents[1]
with tempfile.TemporaryDirectory(prefix="ffrs-capture-lifetime-") as folder:
    build = pathlib.Path(folder)
    if "--baseline" in sys.argv:
        (build / "encounter-under-test.cpp").write_bytes(subprocess.check_output(["git", "show", "f0595ed:cpp/encounter.cpp"], cwd=root))
    else:
        shutil.copy2(root / "cpp/encounter.cpp", build / "encounter-under-test.cpp")
    for name in ("utils.hpp", "test.cpp"):
        shutil.copy2(root / "tests/capture_lifetime" / name, build / name)
    subprocess.run(["cl", "/nologo", "/std:c++20", "/EHsc", "/W4", "/O2", "/MT", "test.cpp", "/Fe:test.exe"], cwd=build, check=True)
    result = subprocess.run([str(build / "test.exe")], cwd=build, capture_output=True, text=True)
    print(result.stdout, end="")
    if "--baseline" in sys.argv:
        if result.returncode != 1 or "success must release exactly one temporary target" not in result.stderr:
            raise RuntimeError("Expected the original cached-target implementation to fail lifetime regression")
        print("PASS: original implementation fails the lifetime regression as expected")
    else:
        if result.returncode:
            print(result.stderr, end="", file=sys.stderr)
            result.check_returncode()
