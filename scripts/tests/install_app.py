#!/usr/bin/env python3
"""Exercise the real installer with a virtual clock and no desktop/process effects."""

import os
from pathlib import Path
import select
import signal
import subprocess
import tempfile
import unittest


INSTALLER = Path(__file__).resolve().parents[1] / "install-app.sh"


class InstallApp(unittest.TestCase):
    def install(self, exit_tick, automatic=True):
        with tempfile.TemporaryDirectory(prefix="spaceman-install-") as temporary:
            root = Path(temporary)
            source = root / "built/Spaceman.app"
            installed = root / "Applications/Spaceman.app"
            for bundle, version in [(source, "new"), (installed, "old")]:
                (bundle / "Contents/MacOS").mkdir(parents=True)
                (bundle / "version").write_text(version)
                cli = bundle / "Contents/MacOS/spaceman"
                cli.write_text('''#!/bin/sh
case "$2" in
  status) printf '{"mode":"%s"}\\n' "$MODE" ;;
  restart) printf 'restart\\n' >> "$CASE/events" ;;
  resume) printf 'resume\\n' >> "$CASE/events"; printf done > "$CASE/done" ;;
  *) exit 2 ;;
esac
''')
                cli.chmod(0o755)
            # Only redirect the install location; all production control flow runs unchanged.
            script = INSTALLER.read_text()
            self.assertEqual(script.count('install_root="/Applications"'), 1)
            script = script.replace('install_root="/Applications"', 'install_root="$CASE/Applications"')
            prelude = '''
ticks=0
pgrep() { [ "$ticks" -lt "$EXIT_TICK" ] && printf '424242\\n'; }
kill() {
  [ "$1" = -0 ] && [ "$2" = 424242 ] || exit 90
  [ "$ticks" -lt "$EXIT_TICK" ]
}
pkill() { exit 91; }
sleep() { ticks=$((ticks + 1)); }
ditto() { cp -R "$1" "$2"; }
codesign() { :; }
open() {
  if pgrep -x spaceman-agent >/dev/null; then
    printf 'opened-before-exit\\n' >> "$CASE/events"
    return 1
  fi
  printf 'open %s %s\\n' "$ticks" "$(cat "$1/version")" >> "$CASE/events"
  if [ "$MODE" = paused ]; then printf done > "$CASE/done"; fi
}
'''
            test_script = root / "install.sh"
            test_script.write_text(prelude + script)
            os.mkfifo(root / "done")
            done = os.open(root / "done", os.O_RDWR | os.O_NONBLOCK)
            process = subprocess.Popen(
                ["sh", str(test_script), "--bundle", str(source)],
                env={**os.environ, "CASE": temporary, "HOME": temporary,
                     "EXIT_TICK": str(exit_tick), "MODE": "automatic" if automatic else "paused"},
                stdout=subprocess.PIPE, stderr=subprocess.PIPE, start_new_session=True,
            )
            try:
                stdout, stderr = process.communicate(timeout=5)
                recovered = bool(select.select([done], [], [], 1)[0])
                events = (root / "events").read_text().splitlines()
                version = (installed / "version").read_text()
                return process.returncode, recovered, events, version, stderr.decode(), stdout.decode()
            finally:
                try:
                    os.killpg(process.pid, signal.SIGKILL)
                except (ProcessLookupError, PermissionError):
                    # macOS can report EPERM for an already-empty process group.
                    pass
                process.wait()
                os.close(done)

    def test_a_25_second_restore_installs_and_resumes(self):
        code, recovered, events, version, error, _ = self.install(125)
        self.assertEqual(code, 0, error)
        self.assertTrue(recovered)
        self.assertEqual(version, "new")
        self.assertEqual(events, ["restart", "open 125 new", "resume"])

    def test_timeout_reopens_old_bundle_after_exit(self):
        for exit_tick in [301, 450]:
            with self.subTest(exit_tick=exit_tick):
                code, recovered, events, version, error, _ = self.install(exit_tick)
                self.assertEqual(code, 1)
                self.assertIn("installation aborted", error)
                self.assertTrue(recovered, events)
                self.assertEqual(version, "old")
                self.assertEqual(events, ["restart", f"open {exit_tick} old", "resume"])

    def test_timeout_does_not_resume_a_paused_app(self):
        code, recovered, events, version, _, _ = self.install(310, automatic=False)
        self.assertEqual(code, 1)
        self.assertTrue(recovered, events)
        self.assertEqual(version, "old")
        self.assertEqual(events, ["restart", "open 310 old"])

    def test_watcher_gives_up_if_old_process_never_exits(self):
        code, recovered, events, version, _, _ = self.install(5000)
        self.assertEqual(code, 1)
        self.assertFalse(recovered, events)
        self.assertEqual(version, "old")
        self.assertEqual(events, ["restart"])


if __name__ == "__main__":
    unittest.main()
