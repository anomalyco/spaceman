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
    def install(
        self,
        exit_tick,
        automatic=True,
        wait_idle=None,
        screen_locked_cmd=None,
        idle_cmd=None,
        log_restart_tick=False,
    ):
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
  status) printf '{"mode":"%s"}\n' "$MODE" ;;
  restart)
    if [ "${LOG_RESTART_TICK:-0}" = 1 ]; then
      printf 'restart %s\n' "$ticks" >> "$CASE/events"
    else
      printf 'restart\n' >> "$CASE/events"
    fi
    ;;
  resume) printf 'resume\n' >> "$CASE/events"; printf done > "$CASE/done" ;;
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
export ticks
pgrep() { [ "$ticks" -lt "$EXIT_TICK" ] && printf '424242\n'; }
kill() {
  [ "$1" = -0 ] && [ "$2" = 424242 ] || exit 90
  [ "$ticks" -lt "$EXIT_TICK" ]
}
pkill() { exit 91; }
sleep() { ticks=$((ticks + 1)); export ticks; }
ioreg() {
  if [ -n "${TEST_IDLE_CMD:-}" ] && [ "$1" = -c ] && [ "$2" = IOHIDSystem ]; then
    secs=$(sh -c "$TEST_IDLE_CMD")
    printf '      "HIDIdleTime" = %s000000000\n' "$secs"
    return 0
  fi
  command ioreg "$@"
}
ditto() { cp -R "$1" "$2"; }
codesign() { :; }
open() {
  if pgrep -x spaceman-agent >/dev/null; then
    printf 'opened-before-exit\n' >> "$CASE/events"
    return 1
  fi
  printf 'open %s %s\n' "$ticks" "$(cat "$1/version")" >> "$CASE/events"
  if [ "$MODE" = paused ]; then printf done > "$CASE/done"; fi
}
'''
            test_script = root / "install.sh"
            test_script.write_text(prelude + script)
            os.mkfifo(root / "done")
            done = os.open(root / "done", os.O_RDWR | os.O_NONBLOCK)
            args = ["sh", str(test_script), "--bundle", str(source)]
            if wait_idle is not None:
                args.extend(["--wait-idle", str(wait_idle)])
            env = {
                **os.environ,
                "CASE": temporary,
                "HOME": temporary,
                "EXIT_TICK": str(exit_tick),
                "MODE": "automatic" if automatic else "paused",
                "LOG_RESTART_TICK": "1" if log_restart_tick else "0",
            }
            if screen_locked_cmd is not None:
                env["SPACEMAN_INSTALL_SCREEN_LOCKED_CMD"] = screen_locked_cmd
            else:
                env.pop("SPACEMAN_INSTALL_SCREEN_LOCKED_CMD", None)
            if idle_cmd is not None:
                env["TEST_IDLE_CMD"] = idle_cmd
            process = subprocess.Popen(
                args,
                env=env,
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

    def test_wait_idle_waits_while_screen_is_locked_and_proceeds_once_unlocked(self):
        # Already idle (>= 120s) from tick 0, but locked for ticks 0..3; unlocks at tick 4.
        code, recovered, events, version, error, _ = self.install(
            exit_tick=7,
            wait_idle=120,
            idle_cmd="echo 300",
            screen_locked_cmd='[ "$ticks" -lt 4 ]',
            log_restart_tick=True,
        )
        self.assertEqual(code, 0, error)
        self.assertTrue(recovered)
        self.assertEqual(version, "new")
        self.assertEqual(events, ["restart 4", "open 7 new", "resume"])

    def test_wait_idle_waits_for_fresh_idle_after_unlock(self):
        # Locked on ticks 0..2 while idle=300; unlocks at tick 3 and user activity resets
        # idle to 0 on ticks 3..5; idle reaches 120 again at tick 6.
        code, recovered, events, version, error, _ = self.install(
            exit_tick=9,
            wait_idle=120,
            idle_cmd='if [ "$ticks" -lt 3 ] || [ "$ticks" -ge 6 ]; then echo 180; else echo 10; fi',
            screen_locked_cmd='[ "$ticks" -lt 3 ]',
            log_restart_tick=True,
        )
        self.assertEqual(code, 0, error)
        self.assertTrue(recovered)
        self.assertEqual(version, "new")
        self.assertEqual(events, ["restart 6", "open 9 new", "resume"])


if __name__ == "__main__":
    unittest.main()
