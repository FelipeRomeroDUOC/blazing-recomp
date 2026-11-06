#!/usr/bin/env python3
"""
build_apk_summary.py — Ejecutor determinista de compilación y empaquetado (reemplaza a APK Builder).
Ejecuta `scripts/package_apk.ps1` o `scripts/diagnostico/compilar_recomp_aot.ps1` y devuelve
en JSON el código de salida, la ruta relativa del artefacto, el tamaño en MB y el primer error.

Uso:
    python scripts/build_apk_summary.py              # Empaqueta el Fat APK
    python scripts/build_apk_summary.py --aot        # Compila los bloques AOT C11
"""

from __future__ import annotations

import json
import os
import re
import subprocess
import sys
from pathlib import Path

RAIZ = Path(__file__).resolve().parent.parent


def main() -> int:
    try:
        sys.stdout.reconfigure(encoding="utf-8", errors="replace")
    except Exception:
        pass

    is_aot = "--aot" in sys.argv
    script_rel = (
        "scripts/diagnostico/compilar_recomp_aot.ps1"
        if is_aot
        else "scripts/package_apk.ps1"
    )
    cmd = ["powershell", "-ExecutionPolicy", "Bypass", "-File", script_rel]
    proc = subprocess.run(
        cmd,
        cwd=str(RAIZ),
        capture_output=True,
        text=True,
        encoding="utf-8",
        errors="replace",
    )

    combined = (proc.stdout or "") + "\n" + (proc.stderr or "")
    errors = re.findall(r"^(?:error(?:\[E\d+\])?:|fatal error:|ERROR:)[^\n]+", combined, flags=re.M | re.I)

    artifacts = []
    for cand in sorted(RAIZ.glob("target/**/*.apk")) if not is_aot else sorted(RAIZ.glob("target/**/*.so")):
        rel = str(cand.relative_to(RAIZ)).replace("\\", "/")
        size_mb = round(os.path.getsize(cand) / (1024 * 1024), 2)
        artifacts.append({"path": rel, "size_mb": size_mb})

    summary = {
        "cmd": " ".join(cmd),
        "exit": proc.returncode,
        "status": "ok" if proc.returncode == 0 else "failed",
        "artifacts": artifacts,
        "first_error": errors[0] if errors else None,
    }
    print(json.dumps(summary, ensure_ascii=False, indent=2))
    return proc.returncode


if __name__ == "__main__":
    raise SystemExit(main())
