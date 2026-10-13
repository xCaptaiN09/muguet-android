import importlib.util
import json
from pathlib import Path
import tempfile
import unittest
import xml.etree.ElementTree as ET

SCRIPT = Path(__file__).with_name("prepare-game.py")
spec = importlib.util.spec_from_file_location("prepare_game", SCRIPT)
prepare = importlib.util.module_from_spec(spec)
spec.loader.exec_module(prepare)


class PrepareGameTests(unittest.TestCase):
    def test_case_insensitive_components_and_source_isolation(self):
        with tempfile.TemporaryDirectory() as root:
            source = Path(root, "source")
            output = Path(root, "output")
            (source / "data/common/data").mkdir(parents=True)
            (source / "data/Env_TEST").mkdir()
            (source / "data/x64/VerityRADIO").mkdir(parents=True)
            (source / "b/build/shaders").mkdir(parents=True)
            (source / "data/Env_TEST/skip.rpf").write_bytes(b"skip")
            (source / "data/x64/VerityRADIO/skip.rpf").write_bytes(b"skip")
            (source / "data/keep.rpf").write_bytes(b"keep")
            (source / "b/build/shaders/index.json").write_text("{}")
            manifest = source / "data/manifest.json"
            manifest.write_text(json.dumps({"mount": "/game/", "files": [
                ["Env_TEST/skip.rpf", 4, 1], ["x64/VerityRADIO/skip.rpf", 4, 1],
                ["keep.rpf", 4, 1]]}))
            dlc = source / "data/common/data/dlcList.xml"
            dlc.write_text('<Paths><Item>platform:\\dlcPacks\\verityRADIO\\</Item><Item>keep</Item></Paths>')
            before_manifest, before_dlc = manifest.read_bytes(), dlc.read_bytes()
            prepare.prepare(source, output)
            self.assertFalse((output / "data/Env_TEST").exists())
            self.assertFalse((output / "data/x64/VerityRADIO").exists())
            self.assertEqual(["keep.rpf"], [entry[0] for entry in json.loads((output / "data/manifest.json").read_text())["files"]])
            self.assertNotIn("verityradio", (output / "data/common/data/dlcList.xml").read_text().lower())
            retained = ET.fromstring((output / "data/common/data/dlcList.xml").read_text())
            self.assertEqual(["keep"], [item.text for item in retained.findall("Item")])
            self.assertEqual(before_manifest, manifest.read_bytes())
            self.assertEqual(before_dlc, dlc.read_bytes())
            self.assertNotEqual(manifest.stat().st_ino, (output / "data/manifest.json").stat().st_ino)
            self.assertNotEqual(dlc.stat().st_ino, (output / "data/common/data/dlcList.xml").stat().st_ino)


if __name__ == "__main__":
    unittest.main()
