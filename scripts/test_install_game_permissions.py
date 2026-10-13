#!/usr/bin/env python3
"""Exercise install-game.sh against a local, filesystem-backed ADB substitute."""
import hashlib
import os
from pathlib import Path
import shlex
import shutil
import subprocess
import sys
import tempfile
import unittest


INSTALLER = Path(__file__).with_name("install-game.sh")
REMOTE = "/sdcard/Android/data/dev.xcaptain.muguet/files"


def fake_adb(args):
    device = Path(os.environ["FAKE_DEVICE_ROOT"])
    command, *parts = args
    if command == "devices":
        print("List of devices attached\nlocal-device\tdevice")
        return 0
    if command == "install":
        print("Success")
        return 0
    if command == "push":
        if parts[0] == "--sync":
            parts.pop(0)
        source, target = parts
        destination = Path(target.replace(REMOTE, str(device)))
        if source.endswith("/."):
            destination.mkdir(parents=True, exist_ok=True)
            shutil.copytree(source[:-2], destination, dirs_exist_ok=True)
            for directory in (destination / "data", destination / "b"):
                for path in [directory, *directory.rglob("*")]:
                    if path.is_dir():
                        path.chmod(0o2770)
            for path in destination.rglob("*"):
                if path.is_file():
                    path.chmod(0o666)
        else:
            destination.parent.mkdir(parents=True, exist_ok=True)
            shutil.copyfile(source, destination)
        return 0
    if command == "shell":
        if parts[:2] == ["df", "-Pk"]:
            print("Filesystem 1024-blocks Used Available Capacity Mounted on\n/dev/fake 99999999 1 99999998 1% /sdcard")
            return 0
        if parts[:2] == ["pm", "path"]:
            print("package:/data/app/fake/base.apk")
            return 0
        if parts[0] == "run-as":
            return 1
        shell = parts[0] if len(parts) == 1 else shlex.join(parts)
        if "chmod" in shell and os.environ.get("FAKE_ADB_DENY_CHMOD") == "1":
            print("chmod: Permission denied", file=sys.stderr)
            return 1
        shell = shell.replace(REMOTE, str(device))
        return subprocess.run(["sh", "-c", shell], check=False).returncode
    raise AssertionError(f"unexpected adb command: {args}")


class InstallerPermissionTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.addCleanup(self.temp.cleanup)
        self.root = Path(self.temp.name)
        self.source = self.root / "source"
        shader = self.source / "b/8b0b5899ed/shaders"
        shader.mkdir(parents=True)
        (self.source / "data/nested").mkdir(parents=True)
        (self.source / "data/manifest.json").write_bytes(b'{"game":"real"}')
        (self.source / "data/nested/asset.bin").write_bytes(b"asset bytes")
        (shader / "index.json").write_bytes(b'{"shader":1}')
        self.device = self.root / "device"
        (self.device / "game").mkdir(parents=True)
        (self.device / "game-code").mkdir()
        (self.device / "unrelated").mkdir()
        (self.device / "unrelated").chmod(0o2770)
        self.apk = self.root / "game.apk"
        self.apk.write_bytes(b"apk")
        self.code = self.root / "game.cwasm"
        self.code.write_bytes(b"trusted code")
        self.bin = self.root / "bin"
        self.bin.mkdir()
        adb = self.bin / "adb"
        adb.write_text(f"#!/bin/sh\nexec python3 {shlex.quote(str(Path(__file__).resolve()))} --fake-adb \"$@\"\n")
        adb.chmod(0o755)
        self.env = os.environ.copy()
        self.env.update({
            "PATH": f"{self.bin}:{self.env['PATH']}",
            "MUGUET_TOOLS_DIR": str(self.root / "tools"),
            "MUGUET_ANDROID_APK": str(self.apk),
            "MUGUET_ANDROID_CONTENT": str(self.source),
            "MUGUET_ANDROID_CODE_ARTIFACT": str(self.code),
            "MUGUET_ANDROID_CODE_SHA256": hashlib.sha256(self.code.read_bytes()).hexdigest(),
            "FAKE_DEVICE_ROOT": str(self.device),
        })

    def stage_restrictive_assets(self, receipt):
        shutil.copytree(self.source, self.device / "game", dirs_exist_ok=True)
        for directory in (self.device / "game/data", self.device / "game/b"):
            for path in [directory, *directory.rglob("*")]:
                if path.is_dir():
                    path.chmod(0o2770)
        for path in (self.device / "game").rglob("*"):
            if path.is_file():
                path.chmod(0o666)
        if receipt:
            result = subprocess.run(
                [sys.executable, str(INSTALLER.with_name("content-receipt.py")), "describe", str(self.source)],
                check=True, capture_output=True, text=True,
            )
            (self.device / "game/.muguet-content-receipt").write_text(result.stdout)

    def run_installer(self, *args, deny_chmod=False):
        env = self.env.copy()
        if deny_chmod:
            env["FAKE_ADB_DENY_CHMOD"] = "1"
        return subprocess.run(["bash", str(INSTALLER), *args], env=env, capture_output=True, text=True)

    def assert_normalized(self):
        for directory in (self.device / "game/data", self.device / "game/b"):
            for path in [directory, *directory.rglob("*")]:
                if path.is_dir():
                    self.assertEqual(0o2775, path.stat().st_mode & 0o7777, str(path))
        self.assertEqual(b"asset bytes", (self.device / "game/data/nested/asset.bin").read_bytes())
        self.assertEqual(b'{"shader":1}', (self.device / "game/b/8b0b5899ed/shaders/index.json").read_bytes())
        self.assertEqual(0o666, (self.device / "game/data/nested/asset.bin").stat().st_mode & 0o777)
        self.assertEqual(0o2770, (self.device / "unrelated").stat().st_mode & 0o7777)

    def test_matching_receipt_still_normalizes_nested_directories(self):
        self.stage_restrictive_assets(receipt=True)
        result = self.run_installer()
        self.assertEqual(0, result.returncode, result.stderr)
        self.assertIn("skipping bulk transfer", result.stdout)
        self.assert_normalized()

    def test_adoption_normalizes_nested_directories(self):
        self.stage_restrictive_assets(receipt=False)
        result = self.run_installer("--adopt-existing-assets")
        self.assertEqual(0, result.returncode, result.stderr)
        self.assertIn("Adopting existing game content", result.stdout)
        self.assert_normalized()

    def test_transfer_normalizes_pushed_nested_directories(self):
        result = self.run_installer()
        self.assertEqual(0, result.returncode, result.stderr)
        self.assertIn("Transferring prepared content", result.stdout)
        self.assert_normalized()

    def test_repair_normalizes_pushed_nested_directories(self):
        self.stage_restrictive_assets(receipt=False)
        result = self.run_installer("--repair-assets")
        self.assertEqual(0, result.returncode, result.stderr)
        self.assertIn("Transferring prepared content", result.stdout)
        self.assert_normalized()

    def test_denied_normalization_fails_without_receipt_or_ready_message(self):
        result = self.run_installer(deny_chmod=True)
        self.assertNotEqual(0, result.returncode)
        self.assertIn("normalize", result.stderr.lower())
        self.assertNotIn("Install ready", result.stdout)
        self.assertFalse((self.device / "game/.muguet-content-receipt").exists())


if __name__ == "__main__":
    if sys.argv[1:2] == ["--fake-adb"]:
        sys.exit(fake_adb(sys.argv[2:]))
    unittest.main()
