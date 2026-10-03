# Copyright (c) Jack Asher
# SPDX-License-Identifier: MPL-2.0

"""Check proposed or committed content against the repository usage policy."""

import argparse
import gzip
import io
import re
import subprocess
import sys
from pathlib import PurePosixPath


PATTERN = re.compile(
    rb"(?:py)?sw(?:eph|isseph)|swiss[\s_-]+(?:ephemeris|eph)"
    rb"|\bswe_[a-z0-9_]+\b|sidereal_reference[.]csv",
    re.IGNORECASE,
)
PROSE_SUFFIXES = {".md", ".mdx", ".rst", ".adoc", ".asciidoc"}
PROSE_NAMES = {"LICENSE", "COPYING", "NOTICE", "THIRD_PARTY_NOTICES"}
MAX_EXPANDED_BYTES = 32 * 1024 * 1024


def git(*args):
    return subprocess.check_output(["git", *args], stderr=subprocess.PIPE)


def entries(tree):
    command = ("ls-tree", "-rz", "--full-tree", tree) if tree else ("ls-files", "--stage", "-z")
    for entry in git(*command).split(b"\0"):
        if not entry:
            continue
        metadata, path = entry.split(b"\t", 1)
        fields = metadata.split()
        if tree:
            mode, kind, oid = fields
            if kind != b"blob":
                raise ValueError("unscanned Git object: " + repr(path))
        else:
            mode, oid, stage = fields
            if stage != b"0" or mode == b"160000":
                raise ValueError("unmerged or external Git entry: " + repr(path))
        yield path, oid.decode("ascii")


def is_prose(path):
    name = PurePosixPath(path.decode("utf-8", errors="surrogateescape"))
    return name.suffix.lower() in PROSE_SUFFIXES or name.name.upper() in PROSE_NAMES


def content(path, oid):
    data = git("cat-file", "blob", oid)
    if path.lower().endswith(b".gz") or data.startswith(b"\x1f\x8b"):
        with gzip.GzipFile(fileobj=io.BytesIO(data)) as archive:
            data = archive.read(MAX_EXPANDED_BYTES + 1)
        if len(data) > MAX_EXPANDED_BYTES:
            raise ValueError("compressed fixture exceeds scan limit: " + repr(path))
    return data


def violations(tree):
    for path, oid in entries(tree):
        if is_prose(path):
            continue
        label = repr(path.decode("utf-8", errors="surrogateescape"))
        if PATTERN.search(path):
            yield f"{label}: prohibited filename"
        data = content(path, oid)
        for match in PATTERN.finditer(data):
            line = data.count(b"\n", 0, match.start()) + 1
            yield f"{label}:{line}: prohibited reference"


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    selection = parser.add_mutually_exclusive_group(required=True)
    selection.add_argument("--index", action="store_true", help="check staged content")
    selection.add_argument("--tree", help="check a committed tree, for example HEAD")
    args = parser.parse_args()
    try:
        tree = git("rev-parse", "--verify", "--end-of-options", args.tree + "^{tree}").decode().strip() if args.tree else None
        found = list(violations(tree))
    except (subprocess.CalledProcessError, OSError, ValueError, EOFError) as error:
        print(f"Usage policy check could not complete: {error}", file=sys.stderr)
        return 2
    if found:
        print("Commit rejected by the usage policy in AGENTS.md:", file=sys.stderr)
        print("\n".join(found), file=sys.stderr)
        return 1
    print("Repository usage policy check passed")
    return 0


if __name__ == "__main__":
    sys.exit(main())
