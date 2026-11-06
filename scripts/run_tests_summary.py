#!/usr/bin/env python3
"""
run_tests_summary.py — Ejecutor determinista de pruebas (reemplaza al subagente Test Runner).
Ejecuta `cargo test` con los argumentos recibidos, distingue errores de compilación de fallos
de test, cuenta los tests pasados/fallados y emite un JSON compacto de 4 campos sin consumir cuota de LLM.

Uso:
    python scripts/run_tests_summary.py --test subagentes_dsh_tests
    python scripts/run_tests_summary.py --lib audio
"""

from __future__ import annotations

import json
import re
import subprocess
import sys
from pathlib import Path

RAIZ = Path(__file__).resolve().parent.parent


def parse_cargo_test_output(stdout: str, stderr: str, exit_code: int) -> dict:
    combined = (stdout or "") + "\n" + (stderr or "")
    compile_errors = re.findall(r"^error(?:\[E\d+\])?:[^\n]+", combined, flags=re.M)

    passed = sum(int(m) for m in re.findall(r"test result: \w+\. (\d+) passed;", stdout or ""))
    failed = sum(int(m) for m in re.findall(r"test result: \w+\. \d+ passed; (\d+) failed;", stdout or ""))
    failed_tests = re.findall(r"^test (\S+) \.\.\. FAILED$", stdout or "", flags=re.M)

    if exit_code == 0:
        status = "passed"
    elif compile_errors and not failed_tests and passed == 0:
        status = "compile_error"
    else:
        status = "test_failed"

    return {
        "cmd": "cargo test " + " ".join(sys.argv[1:]).strip(),
        "exit": exit_code,
        "status": status,
        "passed": passed,
        "failed": failed,
        "failed_tests": failed_tests[:20],
        "compile_errors": compile_errors[:10],
    }


def main() -> int:
    try:
        sys.stdout.reconfigure(encoding="utf-8", errors="replace")
    except Exception:
        pass

    cmd = ["cargo", "test", *sys.argv[1:]]
    proc = subprocess.run(
        cmd,
        cwd=str(RAIZ),
        capture_output=True,
        text=True,
        encoding="utf-8",
        errors="replace",
    )
    summary = parse_cargo_test_output(proc.stdout, proc.stderr, proc.returncode)
    print(json.dumps(summary, ensure_ascii=False, indent=2))
    return proc.returncode


if __name__ == "__main__":
    raise SystemExit(main())
