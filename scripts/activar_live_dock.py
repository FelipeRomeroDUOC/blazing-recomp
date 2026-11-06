#!/usr/bin/env python3
"""
activar_live_dock.py — Script determinista para activar el Live Agents Dock y el Previsualizador
(reemplaza los pasos manuales de la skill `live-dock`).

Uso:
    python scripts/activar_live_dock.py [directorio_artefactos]
"""

from __future__ import annotations

import json
import os
import shutil
import socket
import subprocess
import sys
from pathlib import Path

RAIZ = Path(__file__).resolve().parent.parent
UI_DIR = RAIZ / ".agents" / "ui"
SERVER_SCRIPT = RAIZ / ".agents" / "hooks" / "dock_server.py"
PORT = 48123


def ensure_dock_server() -> bool:
    with socket.socket(socket.AF_INET, socket.SOCK_STREAM) as s:
        s.settimeout(0.5)
        if s.connect_ex(("127.0.0.1", PORT)) == 0:
            return True

    if SERVER_SCRIPT.exists():
        subprocess.Popen(
            [sys.executable, str(SERVER_SCRIPT)],
            cwd=str(RAIZ),
            stdout=subprocess.DEVNULL,
            stderr=subprocess.DEVNULL,
            creationflags=subprocess.CREATE_NO_WINDOW if sys.platform == "win32" else 0,
        )
        return True
    return False


def main() -> int:
    server_ok = ensure_dock_server()
    artifact_dir = Path(sys.argv[1]) if len(sys.argv) > 1 else None
    copied = []

    if artifact_dir and UI_DIR.exists():
        artifact_dir.mkdir(parents=True, exist_ok=True)
        for src_name, dst_name in [
            ("dock_widget.html", "agents_dock_widget.html"),
            ("proposal_preview.html", "proposal_preview.html"),
        ]:
            src = UI_DIR / src_name
            if src.exists():
                dst = artifact_dir / dst_name
                shutil.copy2(src, dst)
                copied.append(str(dst).replace("\\", "/"))

    print(
        json.dumps(
            {
                "server_running": server_ok,
                "port": PORT,
                "artifacts": copied,
            },
            indent=2,
        )
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
