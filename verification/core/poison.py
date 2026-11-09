#!/usr/bin/env python3
"""Reusable runner-free environmental independence witness (no tmux/SSH)."""
import argparse
import json
import os
from pathlib import Path
import shlex
import shutil
import subprocess
import tempfile


def scratch_root():
    """Owned fixture scratch root: TTA_TEST_TMPDIR overrides; /tmp/opencode
    when this user can create directories there; otherwise the system default."""
    override = os.environ.get("TTA_TEST_TMPDIR")
    if override:
        return override
    try:
        with tempfile.TemporaryDirectory(prefix="tta-scratch-probe-", dir="/tmp/opencode"):
            return "/tmp/opencode"
    except OSError:
        return None


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--binary", type=Path, default=Path("target/debug/tta-core-witness"))
    parser.add_argument("--report", type=Path)
    parser.add_argument("--trace", action="store_true", help="also trace syscalls if strace is available")
    args = parser.parse_args()
    binary = args.binary.resolve(strict=True)
    source = Path(__file__).resolve().parents[2]
    trace_available = bool(shutil.which("strace"))
    with tempfile.TemporaryDirectory(prefix="tta-pure-poison-", dir=scratch_root()) as temp:
        root = Path(temp)
        commands = root / "bin"
        commands.mkdir()
        marker = root / "unexpected-execution"
        for name in ["git", "tmux", "ssh", "rekon", "jj", "pnpm"]:
            script = commands / name
            script.write_text(f"#!/bin/sh\nprintf '%s\\n' {shlex.quote(name)} >> {shlex.quote(str(marker))}\nexit 97\n")
            script.chmod(0o700)
        bad_config = root / "bad-config.toml"
        bad_config.write_text("invalid [ deliberately invalid configuration\n")
        outputs = []
        trace_text = None
        for label in ["a", "b"]:
            cwd = root / label
            cwd.mkdir()
            env = {
                "PATH": str(commands), "HOME": str(cwd / "missing-home"),
                "XDG_CONFIG_HOME": str(cwd / "missing-config"),
                "TTA_CONFIG": str(bad_config), "TTA_ROOT": str(cwd / "missing-projects"),
                "PROJECTS_DIR": "contradictory-relative-projects", "AUTHOR": f"poison-{label}",
                "LICENSE": f"poison-license-{label}", "TTA_BACKEND": "invalid-backend",
                "TTA_SOCKET_PATH": str(cwd / "never-connect.sock"),
                "TMUX": str(cwd / "never-connect.sock") + ",123,0",
                "TTA_TMUX": str(commands / "tmux"), "TTA_TMUX_CONFIG": str(bad_config),
            }
            result = subprocess.run([str(binary)], cwd=cwd, env=env, capture_output=True, timeout=10, check=True)
            assert not result.stderr, result.stderr
            assert not marker.exists(), "pure witness executed a fixture program"
            outputs.append(result.stdout)
            if label == "a" and args.trace and trace_available:
                trace = root / "syscalls.log"
                subprocess.run([shutil.which("strace"), "-f", "-qq", "-e", "trace=%file,%network,%process", "-o", str(trace), str(binary)], cwd=cwd, env=env, capture_output=True, timeout=10, check=True)
                trace_text = trace.read_text()
                assert '"/target/' not in trace_text, "target path access observed"
                assert "connect(" not in trace_text and "socket(" not in trace_text, "network I/O observed"
                assert trace_text.count("execve(") == 1, "child execution observed"
        assert outputs[0] == outputs[1], "ambient context changed supplied-facts output"
        tree = subprocess.run([shutil.which("cargo"), "tree", "--locked", "-p", "tta-core-witness", "--no-default-features", "-e", "normal,features", "--prefix", "none"], cwd=source, capture_output=True, text=True, timeout=30, check=True).stdout
        for forbidden in ["tta-runner v", "nix v", "rustix v", "bytes v", 'tmax feature "client"', 'tmax feature "control_mode"']:
            assert forbidden not in tree, forbidden
        report = {
            "success": True, "kind": "supplied-facts poisoned-environment witness", "binary": str(binary),
            "scenarios": len(json.loads(outputs[0])), "contexts": 2, "identical_outputs": True,
            "unexpected_execution": False, "runtime_features_absent": True,
            "syscall_trace_requested": args.trace, "syscall_trace_available": trace_available,
            "syscall_trace": trace_text, "dependency_features": tree,
        }
        text = json.dumps(report, indent=2) + "\n"
        if args.report:
            args.report.write_text(text)
        print(text, end="")


if __name__ == "__main__":
    main()
