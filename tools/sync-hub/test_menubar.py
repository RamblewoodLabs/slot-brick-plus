#!/usr/bin/env python3
"""The menu bar app must not be able to see the hub token."""
import pathlib
import unittest

ROOT = pathlib.Path(__file__).resolve().parent
SWIFT = (ROOT / "menubar" / "main.swift").read_text()
INSTALL = (ROOT / "install.sh").read_text()
UNINSTALL = (ROOT / "uninstall.sh").read_text()
PLIST = (ROOT / "menubar" / "Info.plist").read_text()
AGENT = (ROOT / "menubar" / "com.slot.sync.menubar.plist").read_text()


class MenuBarSourceTests(unittest.TestCase):
    def test_the_app_never_names_the_token_or_its_file(self):
        lowered = SWIFT.lower()
        self.assertNotIn("token", lowered)
        self.assertNotIn("config.json", lowered)
        self.assertNotIn("authorization", lowered)

    def test_turn_off_stops_only_the_hub(self):
        self.assertIn('private let hubLabel = "com.slot.sync"', SWIFT)
        self.assertNotIn("com.slot.sync.menubar", SWIFT)
        self.assertIn("bootout", SWIFT)
        self.assertIn("Turn On", SWIFT)
        self.assertIn("Turn Off", SWIFT)
        self.assertIn("Open Sync Folder", SWIFT)
        self.assertIn("/status", SWIFT)
        self.assertIn("index.json", SWIFT)

    def test_the_bundle_is_a_menu_bar_agent(self):
        self.assertIn("<key>LSUIElement</key>", PLIST)
        self.assertIn("<true/>", PLIST)
        self.assertIn("com.slot.sync.menubar", AGENT)
        self.assertIn("RunAtLoad", AGENT)
        self.assertIn("Slot Sync.app", INSTALL)
        self.assertIn("swiftc", INSTALL)
        self.assertIn("com.slot.sync.menubar", INSTALL)
        self.assertIn("com.slot.sync.menubar", UNINSTALL)
        self.assertIn("Slot Sync.app", UNINSTALL)


if __name__ == "__main__":
    unittest.main()
