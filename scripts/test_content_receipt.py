import importlib.util
from pathlib import Path
import shutil
import tempfile
import unittest

SCRIPT = Path(__file__).with_name("content-receipt.py")
spec = importlib.util.spec_from_file_location("content_receipt", SCRIPT)
receipt = importlib.util.module_from_spec(spec)
spec.loader.exec_module(receipt)


class ContentReceiptTests(unittest.TestCase):
    def test_identity_changes_when_same_size_manifest_content_changes(self):
        with tempfile.TemporaryDirectory() as root:
            tree = Path(root)
            (tree / "data").mkdir()
            (tree / "data/manifest.json").write_bytes(b'{"a":1}')
            (tree / "asset.rpf").write_bytes(b"1234")
            first = receipt.describe_tree(tree)
            self.assertEqual((2, 11), (first.count, first.bytes))
            (tree / "data/manifest.json").write_bytes(b'{"b":1}')
            second = receipt.describe_tree(tree)
            self.assertNotEqual(first.identity, second.identity)
            self.assertEqual((first.count, first.bytes), (second.count, second.bytes))

    def test_interrupted_tree_cannot_be_adopted_or_marked_complete(self):
        expected = receipt.Receipt("a" * 64, 4, 100)
        self.assertEqual("transfer", receipt.plan(None, expected, 0, 0, False, "normal"))
        with self.assertRaises(ValueError):
            receipt.plan(None, expected, 2, 50, False, "normal")
        with self.assertRaises(ValueError):
            receipt.plan(None, expected, 2, 50, False, "adopt")
        self.assertEqual("transfer", receipt.plan(None, expected, 2, 50, False, "repair"))
        self.assertEqual("adopt", receipt.plan(None, expected, 4, 100, True, "adopt"))
        self.assertEqual("reuse", receipt.plan(expected.serialize(), expected, 4, 100, True, "normal"))
        with self.assertRaises(ValueError):
            receipt.plan(expected.serialize(), expected, 2, 50, False, "normal")
        with self.assertRaises(ValueError):
            receipt.plan("mismatched receipt", expected, 4, 100, True, "adopt")

    def test_equal_count_and_bytes_with_wrong_path_cannot_be_adopted(self):
        with tempfile.TemporaryDirectory() as root:
            tree = Path(root)
            (tree / "a.rpf").write_bytes(b"1234")
            (tree / "b.rpf").write_bytes(b"abcdef")
            self.assertEqual((2, 10, True), receipt.compare_remote(tree, ["./a.rpf 4\n", "./b.rpf 6\n"]))
            self.assertEqual((2, 10, False), receipt.compare_remote(tree, ["./a.rpf 4\n", "./wrong.rpf 6\n"]))
            self.assertEqual((2, 10, True), receipt.compare_remote(tree, ["./a.rpf 4\n", "./b.rpf 6\n", "./.muguet-content-receipt 80\n"]))

    def test_interrupted_receipt_write_does_not_make_complete_tree_incomplete(self):
        with tempfile.TemporaryDirectory() as root:
            source, staged = Path(root, "source"), Path(root, "staged")
            source.mkdir()
            staged.mkdir()
            (source / "asset.rpf").write_bytes(b"game")
            shutil.copy2(source / "asset.rpf", staged / "asset.rpf")
            (staged / ".muguet-content-receipt.part").write_bytes(b"unfinished receipt")
            lines = [f"./{path.relative_to(staged).as_posix()} {path.stat().st_size}\n"
                     for path in staged.rglob("*") if path.is_file()]
            self.assertEqual((1, 4, True), receipt.compare_remote(source, lines))
            self.assertEqual(receipt.describe_tree(source), receipt.describe_tree(staged))
            (staged / "nested").mkdir()
            (staged / "nested/.muguet-content-receipt.part").write_bytes(b"unrelated")
            lines = [f"./{path.relative_to(staged).as_posix()} {path.stat().st_size}\n"
                     for path in staged.rglob("*") if path.is_file()]
            self.assertEqual((2, 13, False), receipt.compare_remote(source, lines))


if __name__ == "__main__":
    unittest.main()
