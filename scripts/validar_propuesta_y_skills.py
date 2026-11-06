#!/usr/bin/env python3
"""
validar_propuesta_y_skills.py — Validador determinista de invariantes de skills:
1. Propuestas Nivel 2 (`propuesta-verificable` y `Proposal Reviewer`):
   - Sin marcadores de plantilla (`[nombre corto]`, `[por rellenar]`, `[TODO]`, `<!-- PLANTILLA`) fuera de bloques de código.
   - Casilla `Revisión humana` sin pre-marcar por el agente (`✔ | ✘ | ?` o `ninguno`).
   - Comprobaciones en formato Dado / Cuando / Entonces.
   - Máximo 3 suposiciones (`S1..S3`) y máximo 3 reglas (`R1..R3`).
2. Guías de estudio (`explicar-codigo-documento`):
   - Bloques de código citados de máximo 15 líneas y acompañados de sección `Tu turno`.

Uso:
    python scripts/validar_propuesta_y_skills.py <ruta-archivo.md>
"""

from __future__ import annotations

import re
import sys
from pathlib import Path


TEMPLATE_LEAKS = re.compile(
    r"\[(?:nombre corto|una frase|entrada|por rellenar|TODO|pregunta o decisi[oó]n|lo completa el agente)\]|<!--\s*PLANTILLA",
    re.IGNORECASE,
)


def quitar_codigo_inline_y_bloques(texto: str) -> str:
    sin_bloques = re.sub(r"```[\s\S]*?```", "", texto)
    return re.sub(r"`[^`\n]+`", "", sin_bloques)


def validar_propuesta_n2(texto: str, nombre: str) -> list[str]:
    errores: list[str] = []
    texto_prosa = quitar_codigo_inline_y_bloques(texto)

    # 1. Restos de plantilla en la prosa (excluyendo citas en backticks)
    m_leak = TEMPLATE_LEAKS.search(texto_prosa)
    if m_leak:
        errores.append(f"{nombre}: Contiene marcador de plantilla sin rellenar: {m_leak.group(0)!r}")

    # 2. Límites de Suposiciones (máx 3) y Reglas (máx 3) por identificador único
    suposiciones = set(re.findall(r"^S(\d+)\s*·", texto, flags=re.M))
    if len(suposiciones) > 3:
        errores.append(f"{nombre}: Declara {len(suposiciones)} suposiciones (máximo permitido: 3)")

    reglas = set(re.findall(r"^R(\d+)\s*·", texto, flags=re.M))
    if len(reglas) > 3:
        errores.append(f"{nombre}: Declara {len(reglas)} reglas (máximo permitido: 3)")

    # 3. Formato Dado / Cuando / Entonces en Comprobaciones
    if "## Detalle para verificar" in texto and "### Comprobaciones" in texto:
        for req in ("Dado que", "Cuando", "Entonces"):
            if req not in texto:
                errores.append(f"{nombre}: Falta cláusula '{req}' en las comprobaciones")

    # 4. Revisión humana: si está pendiente o aprobada (no revisada por humano), no debe tener veredicto pre-elegido sin la casilla completa
    estado_m = re.search(r"Estado:\s*([^\n]+)", texto)
    estado = estado_m.group(1).strip().lower() if estado_m else ""
    rev_m = re.search(r"^##+\s*Revisi[oó]n humana\s*$([\s\S]*)", texto, flags=re.M)
    if rev_m and ("pendiente" in estado or "aprobada" in estado):
        cuerpo_rev = rev_m.group(1)
        tiene_casilla_intacta = "✔ | ✘ | ?" in cuerpo_rev
        dice_ninguno = bool(re.search(r"\bningun[oa]\b", cuerpo_rev, re.I))
        if not tiene_casilla_intacta and not dice_ninguno:
            errores.append(
                f"{nombre}: 'Revisión humana' debe mantener '✔ | ✘ | ?' sin elegir o indicar 'ninguno'"
            )

    return errores


def validar_guia_estudio(texto: str, nombre: str) -> list[str]:
    errores: list[str] = []
    bloques = re.findall(r"```\w*\n([\s\S]*?)```", texto)
    for idx, b in enumerate(bloques, 1):
        lineas = b.strip("\n").splitlines()
        if len(lineas) > 15:
            errores.append(f"{nombre}: Bloque de código #{idx} tiene {len(lineas)} líneas (máximo 15)")
    if bloques and texto.lower().count("tu turno") < 1:
        errores.append(f"{nombre}: Guía de estudio sin sección 'Tu turno' (estilo rubber-duck)")
    return errores


def validar_archivo(ruta: Path) -> list[str]:
    if not ruta.exists() or ruta.name == "README.md":
        return []
    texto = ruta.read_text(encoding="utf-8", errors="replace")
    norm = str(ruta).replace("\\", "/").lower()

    if "propuestas/" in norm or texto.lstrip().startswith("# Propuesta:"):
        if "## Para decidir" in texto or "## Revisión humana" in texto:
            return validar_propuesta_n2(texto, ruta.name)
    if "guia-estudio" in norm or "Tu turno" in texto:
        return validar_guia_estudio(texto, ruta.name)
    return []


def main() -> int:
    try:
        sys.stdout.reconfigure(encoding="utf-8", errors="replace")
        sys.stderr.reconfigure(encoding="utf-8", errors="replace")
    except Exception:
        pass

    rutas = [Path(a) for a in sys.argv[1:]]
    if not rutas:
        raiz = Path(__file__).resolve().parent.parent
        rutas = [raiz / "docs" / "propuestas" / "2026-11-06_reglas-deterministas-agentes-skills.md"]

    todos_errores: list[str] = []
    for r in rutas:
        todos_errores.extend(validar_archivo(r))

    if todos_errores:
        for e in todos_errores:
            print(f"ERROR: {e}", file=sys.stderr)
        return 1

    print(f"OK: {len(rutas)} archivo(s) validado(s) sin errores deterministas.")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
