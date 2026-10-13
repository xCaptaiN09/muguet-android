#!/usr/bin/env python3
"""Prepare a separate, source-preserving Android content tree."""
import argparse
import json
import os
from pathlib import Path
import shutil
import tempfile
import xml.etree.ElementTree as ET

SKIP = {"env_test", "verityradio"}
DEFAULT_SOURCE = Path.home() / ".local/share/muguet-android/game-files/full-game-v1/mirror/playgta5.com"
DEFAULT_OUTPUT = Path.home() / ".local/share/muguet-android/prepared/full-game-v1"
MANIFEST = Path("data/manifest.json")
DLC = Path("data/common/data/dlcList.xml")


def skipped(path):
    return any(part.lower() in SKIP for part in Path(path).parts)


def filtered_manifest(source):
    document = json.loads((source / MANIFEST).read_text())
    if not isinstance(document.get("files"), list):
        raise ValueError("manifest has no files list")
    document["files"] = [item for item in document["files"] if not skipped(item[0])]
    return document


def filtered_dlc(source):
    root = ET.parse(source / DLC).getroot()
    for parent in root.iter():
        for item in list(parent):
            if item.tag == "Item" and item.text and any(
                component.lower() in SKIP
                for component in item.text.replace("/", "\\").split("\\")
            ):
                parent.remove(item)
    return ET.tostring(root, encoding="unicode")


def prepared_valid(output, manifest, dlc):
    try:
        if json.loads((output / MANIFEST).read_text()) != manifest:
            return False
        if ET.canonicalize((output / DLC).read_text(), strip_text=True) != ET.canonicalize(dlc, strip_text=True):
            return False
        if not any((output / "b").glob("*/shaders/index.json")):
            return False
        return all((output / "data" / item[0]).is_file() for item in manifest["files"])
    except (OSError, ValueError, ET.ParseError):
        return False


def prepare(source, output):
    source, output = source.resolve(), output.resolve()
    if source == output or source in output.parents or output in source.parents:
        raise ValueError("source and output must be separate trees")
    manifest = filtered_manifest(source)
    dlc = filtered_dlc(source)
    if output.exists():
        if prepared_valid(output, manifest, dlc):
            return "already prepared"
        raise FileExistsError(f"existing output is incomplete or differs: {output}")
    output.parent.mkdir(parents=True, exist_ok=True)
    staging = Path(tempfile.mkdtemp(prefix=output.name + ".", dir=output.parent))
    try:
        for root, dirs, files in os.walk(source):
            root = Path(root)
            relative = root.relative_to(source)
            dirs[:] = [name for name in dirs if not skipped(relative / name)]
            (staging / relative).mkdir(parents=True, exist_ok=True)
            for name in files:
                relative_file = relative / name
                if skipped(relative_file) or relative_file in (MANIFEST, DLC):
                    continue
                original = root / name
                if original.is_symlink():
                    raise ValueError(f"source symlink is unsupported: {original}")
                destination = staging / relative_file
                try:
                    os.link(original, destination)
                except OSError:
                    shutil.copy2(original, destination)
        (staging / MANIFEST).write_text(json.dumps(manifest, separators=(",", ":")))
        (staging / DLC).write_text(dlc)
        if not prepared_valid(staging, manifest, dlc):
            raise ValueError("prepared tree validation failed")
        staging.rename(output)
        return "prepared"
    finally:
        if staging.exists():
            shutil.rmtree(staging)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--source", type=Path, default=DEFAULT_SOURCE)
    parser.add_argument("--output", type=Path, default=DEFAULT_OUTPUT)
    args = parser.parse_args()
    print(prepare(args.source, args.output))


if __name__ == "__main__":
    main()
