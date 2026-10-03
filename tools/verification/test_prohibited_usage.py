# Copyright (c) Jack Asher
# SPDX-License-Identifier: MPL-2.0

"""Exercise the actual hook in temporary Git repositories."""

import gzip
import os
import shutil
import subprocess
import tempfile
from pathlib import Path


ROOT = Path(__file__).resolve().parents[2]
TOKEN = b"sw" + b"eph"


def run(root, *args, expected=0):
    result = subprocess.run(args, cwd=root, capture_output=True, env={
        **os.environ, "GIT_CONFIG_GLOBAL": os.devnull, "GIT_CONFIG_NOSYSTEM": "1",
    })
    assert result.returncode == expected, (args, result.returncode, result.stdout, result.stderr)
    return result


def write(root, path, data):
    target = root / path
    target.parent.mkdir(parents=True, exist_ok=True)
    target.write_bytes(data)


def check(root, expected=0):
    return run(root, "python3", "tools/verification/check_prohibited_usage.py", "--index", expected=expected)


def main():
    with tempfile.TemporaryDirectory(prefix="engine-hook-test-") as directory:
        root = Path(directory)
        run(root, "git", "init", "-q")
        run(root, "git", "config", "user.email", "test@example.invalid")
        run(root, "git", "config", "user.name", "Policy test")
        run(root, "git", "config", "core.hooksPath", ".githooks")
        for path in [".githooks/pre-commit", "tools/verification/check_prohibited_usage.py"]:
            write(root, path, (ROOT / path).read_bytes())
        (root / ".githooks/pre-commit").chmod(0o755)
        write(root, "AGENTS.md", b"Do not use " + TOKEN)
        write(root, "docs/policy.mdx", b"Policy mentions " + TOKEN)
        write(root, "src/clean.rs", b"pub fn value() -> u8 { 1 }\n")
        run(root, "git", "add", "--all")
        run(root, "git", "commit", "-qm", "Clean initial commit")
        run(root, "python3", "tools/verification/check_prohibited_usage.py", "--tree", "HEAD")

        cases = [
            ("src/import.ts", b"import x from '" + TOKEN + b"';"),
            ("Cargo.toml", TOKEN + b' = "1"'),
            ("requirements.lock", b"py" + b"swis" + b"seph==1"),
            ("docs/generate.py", b"import " + b"swis" + b"seph"),
            ("tests/fixture.csv.gz", gzip.compress(b"source: " + TOKEN)),
            ("tests/fixture.csv", b"SWISS" + b" EPHEMERIS"),
            ("src/native.c", b"swe" + b"_calc(0);"),
            ("src/space and\nnewline.ts", TOKEN),
            ("src/" + TOKEN.decode() + ".rs", b""),
        ]
        for path, data in cases:
            write(root, path, data)
            run(root, "git", "add", "--", path)
            assert b"prohibited" in check(root, expected=1).stderr
            run(root, "git", "commit", "-qm", "Must reject", expected=1)
            run(root, "git", "rm", "-qf", "--", path)
        check(root)

        # The index, rather than unstaged disk contents, determines the result.
        write(root, "src/clean.rs", TOKEN)
        check(root)
        run(root, "git", "add", "src/clean.rs")
        write(root, "src/clean.rs", b"clean disk contents")
        check(root, expected=1)
        run(root, "git", "add", "src/clean.rs")
        check(root)
        run(root, "git", "commit", "-qm", "Clean staged version")

        run(root, "git", "mv", "AGENTS.md", "docs/rules.py")
        check(root, expected=1)
        run(root, "git", "rm", "-qf", "docs/rules.py")
        check(root)
        write(root, "tests/broken.gz", b"not gzip")
        run(root, "git", "add", "tests/broken.gz")
        check(root, expected=2)
        run(root, "git", "rm", "-qf", "tests/broken.gz")

        write(root, "src/forbidden.ts", TOKEN)
        run(root, "git", "add", "src/forbidden.ts")
        run(root, "git", "-c", "core.hooksPath=/dev/null", "commit", "-qm", "Bypassed hook")
        run(root, "python3", "tools/verification/check_prohibited_usage.py", "--tree", "HEAD", expected=1)
        run(root, "git", "rm", "-q", "src/forbidden.ts")
        run(root, "git", "commit", "-qm", "Delete prohibited file")
        run(root, "python3", "tools/verification/check_prohibited_usage.py", "--tree", "HEAD")
    print("Usage policy hook integration tests passed")


if __name__ == "__main__":
    main()
