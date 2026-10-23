#!/usr/bin/env python3
"""Reproduce shared-window selection and wrong-client switching, not acceptance.

This preserved trial is deliberately outside Cargo's acceptance suite: a success
means the recorded ambiguity/defect reproduced, not that moving B was correct.
"""

import argparse
import json
import os
from pathlib import Path
import pty
import shlex
import signal
import subprocess
import tempfile
import threading
import time


def wait(predicate, timeout=8):
    deadline = time.monotonic() + timeout
    while time.monotonic() < deadline:
        value = predicate()
        if value:
            return value
        time.sleep(0.02)
    raise AssertionError("timed out waiting for owned tmux observation")


class Terminal:
    def __init__(self, argv, env):
        self.pid, self.fd = pty.fork()
        if self.pid == 0:
            os.execve(argv[0], argv, env)
        self.transcript = bytearray()
        self.thread = threading.Thread(target=self.drain, daemon=True)
        self.thread.start()

    def drain(self):
        try:
            while data := os.read(self.fd, 8192):
                self.transcript.extend(data)
                if len(self.transcript) > 262144:
                    del self.transcript[:-262144]
        except OSError:
            pass

    def write(self, value):
        os.write(self.fd, value.encode())

    def stop(self):
        try:
            os.kill(self.pid, signal.SIGTERM)
            deadline = time.monotonic() + 2
            while time.monotonic() < deadline:
                if os.waitpid(self.pid, os.WNOHANG)[0]:
                    return
                time.sleep(0.02)
            os.kill(self.pid, signal.SIGKILL)
            os.waitpid(self.pid, 0)
        except (ProcessLookupError, ChildProcessError):
            pass
        finally:
            os.close(self.fd)


def exercise(tta, backend, grouped):
    with tempfile.TemporaryDirectory(prefix="tta-view-proof-", dir="/tmp/opencode") as temp:
        root = Path(temp)
        projects = root / "projects"
        for name in ["work", "other"]:
            (projects / name).mkdir(parents=True)
        socket = root / "sock"
        config = root / "tmux.conf"
        config.write_text("set -g status off\nset -g mouse off\n")
        env = {k: v for k, v in os.environ.items() if not k.startswith("TTA_") and k not in ["TMUX", "TMUX_PANE", "PROJECTS_DIR", "ENV", "BASH_ENV"]}
        env.update(HOME=str(root), TERM="xterm-256color", PS1="proof> ", TTA_CONFIG=str(root / "missing-config"))
        base = [tta, "--backend", backend, "--root", str(projects), "--socket-path", str(socket), "--tmux-config", str(config), "--scaffold=false"]
        launch = [*base, *(["--group"] if grouped else []), "work", "--", "/bin/sh", "-i"]
        terminals = []

        def tmux(*args, check=True):
            return subprocess.run(["tmux", "-S", str(socket), "-f", str(config), *args], env=env, capture_output=True, text=True, check=check, timeout=4)

        def clients():
            output = tmux("list-clients", "-F", "#{client_pid}\t#{client_name}\t#{session_name}\t#{session_id}\t#{window_id}\t#{pane_id}", check=False)
            return {int(row[0]): dict(zip(["pid", "name", "session", "session_id", "window", "pane"], row)) for line in output.stdout.splitlines() if len(row := line.split("\t")) == 6}

        try:
            assert not socket.exists()
            first = Terminal(launch, env)
            terminals.append(first)
            wait(lambda: first.pid in clients())
            second = Terminal(launch, env)
            terminals.append(second)
            entered = wait(lambda: rows if first.pid in (rows := clients()) and second.pid in rows else None)
            a, b = entered[first.pid], entered[second.pid]
            assert (a["session_id"] != b["session_id"]) == grouped, entered
            assert a["pane"] == b["pane"], entered
            window = tmux("new-window", "-d", "-P", "-F", "#{window_id}", "-t", "=work:", "/bin/sh", "-i").stdout.strip()
            tmux("select-window", "-t", f'{b["session_id"]}:{window}')
            selected = clients()
            assert selected[second.pid]["window"] == window, selected
            assert (selected[first.pid]["window"] != window) == grouped, selected
            result = dict(grouped=grouped, backend=backend, distinct_sessions=grouped, independent_windows=grouped, entered=entered, after_second_select=selected)
            if grouped:
                tmux("select-window", "-t", f'{b["session_id"]}:{a["window"]}')
                # Both terminal inputs reach the SAME shell/pane. A launches a
                # delayed tta; B is active before its CLI-owned switch resolves.
                first.write("printf '%s\\n' \"$TMUX\" \"$TMUX_PANE\" \"$(tty)\" > " + shlex.quote(str(root / "from-a")) + "\r")
                wait(lambda: (root / "from-a").exists())
                second.write("printf '%s\\n' \"$TMUX\" \"$TMUX_PANE\" \"$(tty)\" > " + shlex.quote(str(root / "from-b")) + "\r")
                wait(lambda: (root / "from-b").exists())
                assert (root / "from-a").read_bytes() == (root / "from-b").read_bytes()
                first.write("sleep 1; " + shlex.join([*base, "other"]) + "; printf done > " + shlex.quote(str(root / "done")) + "\r")
                time.sleep(0.15)
                second.write("\x02:display-message activity-from-B\r")

                def moved():
                    rows = clients()
                    return rows if any(c["session"] == "other" for c in rows.values()) else None

                switched = wait(moved)
                result.update(identical_process_context=(root / "from-a").read_text().splitlines(), requested_from_pid=first.pid, switched_to_other_pid=next(pid for pid, c in switched.items() if c["session"] == "other"), after_inside_switch=switched)
                assert result["switched_to_other_pid"] == second.pid, result
            return result
        except Exception:
            for terminal in terminals:
                print(terminal.transcript.decode(errors="replace"), file=__import__("sys").stderr)
            raise
        finally:
            tmux("kill-server", check=False)
            for terminal in terminals:
                terminal.stop()


if __name__ == "__main__":
    parser = argparse.ArgumentParser()
    parser.add_argument("--tta", required=True)
    parser.add_argument("--backend", choices=["native", "cli"], default="native")
    args = parser.parse_args()
    tta = str(Path(args.tta).resolve())
    print(json.dumps(dict(version=subprocess.check_output(["tmux", "-V"], text=True).strip(), cases=[exercise(tta, args.backend, grouped) for grouped in [False, True]]), indent=2))
