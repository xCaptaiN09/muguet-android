#!/usr/bin/env python3
"""Cheap content identity/completeness metadata for the ADB installer."""
import argparse
import hashlib
from pathlib import Path
import sys
from typing import NamedTuple

MARKER = ".muguet-content-receipt"
MAGIC = "MUGUET_CONTENT_V1"
CONTENT_FILES = {"data/manifest.json", "data/common/data/dlcList.xml"}


class Receipt(NamedTuple):
    identity: str
    count: int
    bytes: int

    def serialize(self):
        return f"{MAGIC} {self.identity} {self.count} {self.bytes}"


def describe_tree(root):
    digest = hashlib.sha256()
    count = total = 0
    for path in sorted(Path(root).rglob("*")):
        if path.is_symlink():
            raise ValueError(f"content tree contains symlink: {path}")
        if not path.is_file() or path.name == MARKER:
            continue
        relative = path.relative_to(root).as_posix()
        size = path.stat().st_size
        count += 1
        total += size
        digest.update(f"{relative}\0{size}\n".encode())
        if relative in CONTENT_FILES or relative.endswith("/shaders/index.json"):
            digest.update(hashlib.sha256(path.read_bytes()).digest())
    return Receipt(digest.hexdigest(), count, total)


def compare_remote(root, lines):
    expected = {
        path.relative_to(root).as_posix(): path.stat().st_size
        for path in Path(root).rglob("*") if path.is_file() and path.name != MARKER
    }
    actual = {}
    count = total = 0
    for line in lines:
        line = line.rstrip("\r\n")
        if not line:
            continue
        name, raw_size = line.rsplit(" ", 1)
        name = name.removeprefix("./")
        if Path(name).name == MARKER:
            continue
        size = int(raw_size)
        if size < 0:
            raise ValueError("negative remote file size")
        actual[name] = size
        count += 1
        total += size
    return count, total, count == len(actual) and actual == expected


def plan(existing, expected, remote_count, remote_bytes, remote_matches, mode):
    if mode == "repair":
        return "transfer"
    if existing and existing.strip() == expected.serialize():
        if remote_matches and (remote_count, remote_bytes) == (expected.count, expected.bytes):
            return "reuse"
        raise ValueError("completed content receipt exists but phone metadata differs; use --repair-assets")
    if existing:
        raise ValueError("existing content receipt is for a different build; use --repair-assets")
    if mode == "adopt":
        if not remote_matches or (remote_count, remote_bytes) != (expected.count, expected.bytes):
            raise ValueError("existing phone content metadata is incomplete; use --repair-assets")
        return "adopt"
    if remote_count == 0:
        return "transfer"
    raise ValueError("unreceipted phone content exists; use --adopt-existing-assets or --repair-assets")


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    sub = parser.add_subparsers(dest="command", required=True)
    describe = sub.add_parser("describe")
    describe.add_argument("root", type=Path)
    compare = sub.add_parser("compare")
    compare.add_argument("root", type=Path)
    choose = sub.add_parser("plan")
    choose.add_argument("expected")
    choose.add_argument("existing")
    choose.add_argument("count", type=int)
    choose.add_argument("bytes", type=int)
    choose.add_argument("matches", choices=["0", "1"])
    choose.add_argument("mode", choices=["normal", "adopt", "repair"])
    args = parser.parse_args()
    if args.command == "describe":
        print(describe_tree(args.root).serialize())
    elif args.command == "compare":
        count, total, matches = compare_remote(args.root, sys.stdin)
        print(count, total, int(matches))
    else:
        fields = args.expected.split()
        if len(fields) != 4 or fields[0] != MAGIC:
            parser.error("invalid expected receipt")
        expected = Receipt(fields[1], int(fields[2]), int(fields[3]))
        print(plan(None if args.existing == "-" else args.existing,
                   expected, args.count, args.bytes, args.matches == "1", args.mode))


if __name__ == "__main__":
    try:
        main()
    except ValueError as error:
        print(error, file=sys.stderr)
        sys.exit(1)
