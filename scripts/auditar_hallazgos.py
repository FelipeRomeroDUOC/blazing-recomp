#!/usr/bin/env python3
"""Audita el registro de hallazgos del BACKLOG contra las propuestas.

Los hallazgos "fuera de alcance" nacen al final de cada propuesta y hasta ahora
no tenían registro: quien buscaba "qué falta" tenía que abrir las 56 propuestas.
Este script cierra ese hueco en las dos direcciones:

  --list   Extrae los hallazgos de todas las propuestas (y los imprime).
  (nada)   Comprueba que el registro de `docs/BACKLOG.md` los cubre todos y que
           los enlaces del registro resuelven a archivos existentes.

La comparación es por huella: los primeros caracteres significativos de cada
hallazgo, normalizados (sin acentos, sin markdown, sin la cola `| impacto:`).
Así el registro puede agrupar y reordenar sin dejar de ser comprobable.

Uso:
    python scripts/auditar_hallazgos.py            # audita
    python scripts/auditar_hallazgos.py --list     # extrae
"""

from __future__ import annotations

import re
import sys
import unicodedata
from itertools import groupby
from pathlib import Path

RAIZ = Path(__file__).resolve().parent.parent
PROPUESTAS = RAIZ / "docs" / "propuestas"
BACKLOG = RAIZ / "docs" / "BACKLOG.md"

# Encabezados que el proyecto ha usado para esta sección.
ENCABEZADOS = ("Fuera de alcance",)
# Longitud de la huella con la que se identifica un hallazgo en el registro.
HUELLA = 42


def normalizar(texto: str) -> str:
    """Minúsculas, sin acentos y sin ruido de markdown."""
    texto = unicodedata.normalize("NFKD", texto)
    texto = "".join(c for c in texto if not unicodedata.combining(c))
    texto = texto.lower()
    texto = re.sub(r"[`*_\[\]()>#]", " ", texto)
    texto = re.sub(r"https?://\S+", " ", texto)
    texto = re.sub(r"[^\w\s]", " ", texto)
    return re.sub(r"\s+", " ", texto).strip()


def sin_cola_de_impacto(texto: str) -> str:
    """Quita el sufijo `| impacto: alto` y variantes."""
    return re.split(r"\|\s*impacto\s*:", texto, maxsplit=1, flags=re.I)[0].strip()


def seccion_fuera_de_alcance(texto: str) -> str:
    """Cuerpo de la sección `Fuera de alcance`, sea `###` o `##`."""
    patron = r"^#+\s*(" + "|".join(ENCABEZADOS) + r")[^\n]*\n(.*?)(?=^#|^---\s*$|\Z)"
    m = re.search(patron, texto, re.M | re.S)
    return m.group(2).strip() if m else ""


def extraer_hallazgos() -> list[tuple[str, str]]:
    """[(propuesta, hallazgo)] de todas las propuestas, en orden.

    Un hallazgo puede ocupar varias líneas: las continuaciones van indentadas
    y se unen al bullet para que el registro cite el texto completo.
    """
    hallazgos: list[tuple[str, str]] = []
    for ruta in sorted(PROPUESTAS.glob("*.md")):
        if ruta.name == "README.md":
            continue
        cuerpo = seccion_fuera_de_alcance(ruta.read_text(encoding="utf-8", errors="replace"))
        if not cuerpo:
            continue
        for linea in cuerpo.splitlines():
            limpia = linea.strip()
            if limpia.startswith("- "):
                texto = limpia[2:].strip()
                if texto.lower().startswith("ningun"):
                    continue
                hallazgos.append((ruta.name, texto))
            elif hallazgos and linea[:1] in (" ", "\t") and limpia and not limpia.startswith("#"):
                propuesta, previo = hallazgos[-1]
                if propuesta == ruta.name:
                    hallazgos[-1] = (propuesta, f"{previo} {limpia}")
    return [(p, re.sub(r"\s+", " ", t).strip()) for p, t in hallazgos]


def huella(texto: str) -> str:
    return normalizar(sin_cola_de_impacto(texto))[:HUELLA]


def linea_registro(propuesta: str, texto: str, con_origen: bool = True) -> str:
    """Una línea del registro: el hallazgo **verbatim**, su impacto y su origen.

    El hallazgo se copia tal cual empieza en la propuesta, sin reescribirlo: el
    registro enlaza, no traslada.
    """
    cuerpo = sin_cola_de_impacto(texto).rstrip()
    if not cuerpo.endswith((".", ":", ")")):
        cuerpo += "."
    origen = f" — origen: [`{propuesta}`](propuestas/{propuesta})" if con_origen else ""
    return f"- {cuerpo} — impacto: {impacto(texto)}{origen}"


def registrar(hallazgos: list[tuple[str, str]]) -> None:
    """Registro plano, un hallazgo por línea, en orden de propuesta."""
    for propuesta, texto in hallazgos:
        print(linea_registro(propuesta, texto))


def seccion_registro(hallazgos: list[tuple[str, str]]) -> None:
    """Sección completa en markdown: agrupada por propuesta y ordenada por impacto."""
    for propuesta, grupo in groupby(hallazgos, key=lambda x: x[0]):
        print(f"#### [`{propuesta}`](propuestas/{propuesta})\n")
        items = sorted(grupo, key=lambda x: ORDEN_IMPACTO.get(impacto(x[1]), 4))
        for _, texto in items:
            print(linea_registro(propuesta, texto, con_origen=False))
        print()


def impacto(texto: str) -> str:
    """Impacto declarado, en cualquiera de las formas que usa el proyecto."""
    m = re.search(r"impacto\s*:?\s*\**\s*([a-zA-ZáéíóúÁÉÍÓÚñÑ]+(?:-[a-zA-Záéíóú]+)?)", texto, re.I)
    if not m:
        return "sin declarar"
    valor = m.group(1).lower()
    return valor if valor in ORDEN_IMPACTO else "sin declarar"


# De más urgente a menos, para ordenar dentro de cada propuesta.
ORDEN_IMPACTO = {"alto": 0, "bajo-medio": 1, "medio": 2, "bajo": 3, "sin declarar": 4}


def enlaces_del_registro(backlog: str) -> list[str]:
    """Enlaces relativos a propuestas dentro del registro de hallazgos."""
    seccion = re.search(
        r"^##\s*Registro de hallazgos.*?$(.*?)(?=^##\s|\Z)", backlog, re.M | re.S | re.I
    )
    if not seccion:
        return []
    return re.findall(r"\]\((propuestas/[^)#]+\.md)\)", seccion.group(1))


def seccion_actual(backlog: str) -> str | None:
    """Cuerpo actual del registro, para poder regenerarlo sin duplicarlo."""
    m = re.search(r"^##\s*Registro de hallazgos.*?$(.*?)(?=^##\s|\Z)", backlog, re.M | re.S | re.I)
    return m.group(1).strip() if m else None


def reemplazar_seccion(hallazgos: list[tuple[str, str]]) -> None:
    """Regenera el registro dentro del BACKLOG, conservando el resto del archivo."""
    import io
    from contextlib import redirect_stdout

    buffer = io.StringIO()
    with redirect_stdout(buffer):
        seccion_registro(hallazgos)
    nueva = buffer.getvalue().strip()
    texto = BACKLOG.read_text(encoding="utf-8")
    actual = seccion_actual(texto)
    if actual is None:
        raise SystemExit("ERROR: no encuentro la sección '## Registro de hallazgos' en el BACKLOG")
    BACKLOG.write_text(texto.replace(actual, nueva), encoding="utf-8")
    print(f"registro regenerado: {len(nueva.splitlines())} líneas")


def main() -> int:
    hallazgos = extraer_hallazgos()
    if "--registro" in sys.argv:
        registrar(hallazgos)
        return 0
    if "--seccion" in sys.argv:
        if "--escribir" in sys.argv:
            reemplazar_seccion(hallazgos)
            return 0
        seccion_registro(hallazgos)
        return 0
    if "--list" in sys.argv:
        for propuesta, texto in hallazgos:
            print(f"{propuesta}\t[{impacto(texto)}]\t{sin_cola_de_impacto(texto)[:150]}")
        print(f"\ntotal: {len(hallazgos)} hallazgos en "
              f"{len({p for p, _ in hallazgos})} propuestas", file=sys.stderr)
        return 0

    if not BACKLOG.exists():
        print(f"ERROR: falta {BACKLOG}")
        return 1
    backlog = BACKLOG.read_text(encoding="utf-8", errors="replace")
    normalizado = normalizar(backlog)

    faltan = [(p, t) for p, t in hallazgos if huella(t) not in normalizado]
    for propuesta, texto in faltan:
        print(f"SIN REGISTRAR [{propuesta}]: {sin_cola_de_impacto(texto)[:90]}")
    if faltan:
        print(f"\nERROR: {len(faltan)} de {len(hallazgos)} hallazgos no están en el registro "
              f"de docs/BACKLOG.md.")
        return 1

    enlaces = enlaces_del_registro(backlog)
    rotos = [e for e in set(enlaces) if not (RAIZ / "docs" / e).exists()]
    for enlace in rotos:
        print(f"ENLACE ROTO en el registro: docs/{enlace}")
    if rotos:
        print(f"\nERROR: {len(rotos)} enlaces del registro no resuelven.")
        return 1

    propuestas = len({p for p, _ in hallazgos})
    print(f"OK: el registro cubre los {len(hallazgos)} hallazgos de {propuestas} propuestas "
          f"y sus {len(set(enlaces))} enlaces resuelven.")
    return 0


if __name__ == "__main__":
    sys.exit(main())
