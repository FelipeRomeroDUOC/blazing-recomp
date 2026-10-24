#!/usr/bin/env python3
"""Regenera el índice de propuestas de `docs/BACKLOG.md` a partir de cada ficha.

Las secciones por estado se reconstruyen desde la línea `Estado:` de cada
propuesta, sin tocar las anotaciones curadas que ya estaban escritas. Los
recuentos del encabezado y de cada título se recalculan.

La lista «Esperando el televisor» es la unión de lo que ya estaba en ella y lo
que la ficha dice que sigue pendiente, menos lo que ya se puede dar por
resuelto (alguien puso la marca o la ficha dice que no hay nada pendiente).
Lo que el script no sabe clasificar se conserva: no borra una entrada por no
entender su formato. El encabezado de esa lista no lleva el número —el
recuento cambia con cada regeneración y el ancla tiene que ser estable para
los enlaces—; el recuento va en la primera línea del cuerpo, y siempre
coincide con el número de entradas de la lista.

El script es idempotente: ejecutarlo dos veces seguidas deja el fichero igual.

Uso:
    python scripts/actualizar_indice_propuestas.py            # escribe el cambio
    python scripts/actualizar_indice_propuestas.py --dry-run  # solo lo enseña
"""

from __future__ import annotations

import re
import sys
from pathlib import Path

RAIZ = Path(__file__).resolve().parent.parent
PROPUESTAS = RAIZ / "docs" / "propuestas"
BACKLOG = RAIZ / "docs" / "BACKLOG.md"

VOCABULARIO = [
    "verificada por agente",
    "cerrada",
    "aprobada",
    "pendiente",
    "discrepada",
    "revisada por humano",
    "modificada",
    "descartada",
]

# Encabezado de la lista «Esperando el televisor». Sin número: el ancla que
# genera tiene que sobrevivir a cada regeneración del índice.
ENCABEZADO_TELEVISOR = (
    "### Esperando el televisor: propuestas con `Revisión humana` pendiente"
)

# Sección del índice -> prefijo de título con el que se anuncia.
SECCIONES = [
    ("### Pendientes", "pendiente", "— sin ejecutar"),
    ("### Aprobadas sin cerrar", "aprobada", "— alcance pactado, trabajo o medida pendientes"),
    ("### Discrepadas", "discrepada", "— la medida contradijo la premisa"),
    ("### Revisada por humano", "revisada por humano", ""),
    ("### Cerradas", "cerrada", ""),
    ("### Verificadas por agente", "verificada por agente", "— con lo medido dentro; varias esperan el televisor"),
    ("### Modificadas", "modificada", "— cambiadas sobre la marcha"),
    ("### Descartadas", "descartada", "— la medición las dejó fuera"),
]


def estado_de(texto: str) -> str | None:
    for linea in texto.splitlines():
        if linea.startswith("Estado:"):
            valor = linea.split(":", 1)[1].strip()
            for v in VOCABULARIO:
                if valor.startswith(v):
                    return v
    return None


CASILLA_SIN_MARCAR = "✔ | ✘ | ?"


def revision_humana(texto: str) -> tuple[bool, bool]:
    """Clasifica la sección «Revisión humana» de una ficha.

    Devuelve `(tiene_sección, sigue_pendiente)`:

    - La casilla de la plantilla sin marcar («✔ | ✘ | ?») significa pendiente.
    - Con marcas ya puestas (✔ o ✘) alguien respondió: está resuelta.
    - Sin marcas, la prosa decide: si dice que no hay nada pendiente
      («ninguno» / «ninguna») está resuelta; si enumera comprobaciones
      humanas, sigue pendiente.
    """
    m = re.search(r"^##+\s*Revisi[oó]n humana\s*$", texto, re.M)
    if not m:
        return False, False
    cuerpo = re.split(r"^##\s", texto[m.end():], maxsplit=1, flags=re.M)[0]
    if CASILLA_SIN_MARCAR in cuerpo:
        return True, True
    if re.search(r"[✔✘]", cuerpo):
        return True, False
    if re.search(r"\bningun[oa]\b", cuerpo, re.IGNORECASE):
        return True, False
    return True, True


def main() -> int:
    dry = "--dry-run" in sys.argv

    fichas: dict[str, str] = {}
    pendientes_tv: list[str] = []
    resueltas_tv: set[str] = set()
    for ruta in sorted(PROPUESTAS.glob("*.md")):
        if ruta.name == "README.md":
            continue
        texto = ruta.read_text(encoding="utf-8", errors="replace")
        estado = estado_de(texto)
        if estado is None:
            print(f"AVISO: {ruta.name} no declara un estado válido", file=sys.stderr)
            continue
        fichas[ruta.name] = estado
        tiene_seccion, pendiente = revision_humana(texto)
        if not tiene_seccion:
            continue
        if pendiente:
            pendientes_tv.append(ruta.name)
        else:
            resueltas_tv.add(ruta.name)

    backlog = BACKLOG.read_text(encoding="utf-8")
    ini = backlog.index("## Índice de Propuestas")
    fin = backlog.index("## Registro de Hallazgos")
    bloque = backlog[ini:fin]

    # Enlaces ya presentes en el índice.
    listadas = set(re.findall(r"\]\(propuestas/([^)]+\.md)\)", bloque))

    # Reparto de lo que falta.
    faltan: dict[str, list[str]] = {v: [] for v in VOCABULARIO}
    for nombre, estado in fichas.items():
        if nombre not in listadas:
            faltan[estado].append(nombre)

    for v in VOCABULARIO:
        faltan[v].sort()

    # 1. Crear las secciones por estado que todavía no existen.
    ancla = re.search(r"### Esperando el televisor", bloque)
    if ancla:
        nuevas = ""
        for titulo, estado, cola in SECCIONES:
            if re.search(re.escape(titulo) + r"(?:\s*\(\d+\))?", bloque):
                continue
            total = sum(1 for e in fichas.values() if e == estado)
            encabezado = f"{titulo} ({total})"
            if cola:
                encabezado += f" {cola}"
            nuevas += encabezado + "\n\n"
        if nuevas:
            bloque = bloque[: ancla.start()] + nuevas + bloque[ancla.start():]

    # 2. Reconstruir cada sección por estado desde la ficha de cada propuesta.
    #    Las anotaciones que ya estaban escritas viajan con su propuesta.
    anotaciones: dict[str, str] = {}
    for linea in re.findall(r"^- \[.*\n", bloque, re.M):
        m = re.match(r"- \[`([^`]+)`\]\([^)]+\)(.*)\n?$", linea)
        if m and m.group(2).strip():
            anotaciones[m.group(1)] = m.group(2).strip()

    for titulo, estado, cola in SECCIONES:
        patron = re.compile(
            r"(" + re.escape(titulo) + r")(?:\s*\((\d+)\))?([^\n]*)\n\n((?:- .*\n)*)", re.M
        )
        m = patron.search(bloque)
        if not m:
            print(f"AVISO: no encuentro la sección {titulo!r}", file=sys.stderr)
            continue
        members = sorted(n for n, e in fichas.items() if e == estado)
        lineas = ""
        for n in members:
            extra = anotaciones.get(n, "")
            sufijo = f" {extra}" if extra else ""
            lineas += f"- [`{n}`](propuestas/{n}){sufijo}\n"
        total = len(members)
        encabezado = f"{titulo} ({total})"
        if cola:
            encabezado += f" {cola}"
        bloque = bloque[: m.start()] + encabezado + "\n\n" + lineas + bloque[m.end():]

    # 2. Lista de las que esperan el televisor.
    #    El encabezado no lleva el número: el recuento cambia con cada
    #    regeneración y el ancla tiene que ser estable para los enlaces.
    m = re.search(
        r"### Esperando el televisor:[^\n]*\n\n((?:[^\n]*\n\n)*)((?:- .*\n)*)",
        bloque,
    )
    if m:
        ya = set(re.findall(r"\]\(propuestas/([^)]+\.md)\)", m.group(2)))
        # La lista es lo curado más lo pendiente que se detecta, menos lo que
        # ya se puede dar por resuelto. Lo que el script no sabe clasificar se
        # conserva: no se borra una entrada por no entender su formato.
        miembros = sorted((ya | set(pendientes_tv)) - resueltas_tv)
        lineas = ""
        for n in miembros:
            extra = anotaciones.get(n, "")
            sufijo = f" {extra}" if extra else ""
            lineas += f"- [`{n}`](propuestas/{n}){sufijo}\n"
        # El texto curado se conserva; solo se regenera la frase del recuento.
        parrafos = [p.strip("\n") for p in m.group(1).split("\n\n") if p.strip()]
        resto = re.sub(r"^Son \d+ en total\.\s*", "", " ".join(parrafos)).strip()
        cuerpo = f"Son {len(miembros)} en total."
        if resto:
            cuerpo += f" {resto}"
        bloque = (
            bloque[: m.start()]
            + ENCABEZADO_TELEVISOR
            + "\n\n"
            + cuerpo
            + "\n\n"
            + lineas
            + bloque[m.end():]
        )

    # 3. Separación canónica: una sola línea en blanco entre secciones.
    bloque = re.sub(r"\n{3,}", "\n\n", bloque)

    # 4. Recuentos del encabezado.
    total = len(fichas)
    reparto = " + ".join(f"{sum(1 for e in fichas.values() if e == v)} `{v}`" for v in VOCABULARIO)
    bloque = re.sub(r"\d+ propuestas en \[`docs/propuestas/`\]", f"{total} propuestas en [`docs/propuestas/`]", bloque)
    bloque = re.sub(r"\*\*\d+ = [^.]+\.\*\*", f"**{total} = {reparto}.**", bloque)

    if dry:
        sys.stdout.reconfigure(encoding="utf-8", errors="replace")
        print(bloque)
        return 0

    BACKLOG.write_text(backlog[:ini] + bloque + backlog[fin:], encoding="utf-8")
    print(f"índice actualizado: {total} propuestas, {len(pendientes_tv)} esperando el televisor")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
