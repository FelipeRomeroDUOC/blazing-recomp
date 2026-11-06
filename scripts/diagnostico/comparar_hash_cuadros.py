#!/usr/bin/env python3
r"""
scripts/diagnostico/comparar_hash_cuadros.py

Compara registros de hash de cuadros entre Android y PC.
Formato de linea esperado:
# hash rel=(\d+) frame=(\d+) ent=([0-9A-Fa-f]+) sal=([0-9A-Fa-f]+) bld0=([0-9A-Fa-f]+) bld50=([0-9A-Fa-f]+) bld100=([0-9A-Fa-f]+) bld150=([0-9A-Fa-f]+) bg3_blend=(\d+)

Campos opcionales de componentes de entrada (al final de la linea):
hc=([0-9A-Fa-f]+) hv=([0-9A-Fa-f]+) ho=([0-9A-Fa-f]+) hp=([0-9A-Fa-f]+) hl=([0-9A-Fa-f]+)
Donde:
  hc: registros por linea (registros_por_linea)
  hv: VRAM
  ho: OAM
  hp: paleta de fin de cuadro (paleta)
  hl: paleta por linea (paleta_lineas)

Codigos de salida:
  0: Coincidencia total (identicos).
  1: Discrepancia encontrada en cuadros.
  2: Error de argumentos, lectura o datos faltantes / archivo vacio.
"""

import argparse
import re
import sys
from typing import Dict, Any

PATTERN = re.compile(
    r"#\s*hash\s+rel=(\d+)\s+frame=(\d+)\s+ent=([0-9A-Fa-f]+)\s+sal=([0-9A-Fa-f]+)\s+"
    r"bld0=([0-9A-Fa-f]+)\s+bld50=([0-9A-Fa-f]+)\s+bld100=([0-9A-Fa-f]+)\s+bld150=([0-9A-Fa-f]+)\s+"
    r"bg3_blend=(\d+)"
)

EXTRA = re.compile(
    r"hc=([0-9A-Fa-f]+)\s+hv=([0-9A-Fa-f]+)\s+ho=([0-9A-Fa-f]+)\s+hp=([0-9A-Fa-f]+)\s+hl=([0-9A-Fa-f]+)"
)


def parsear_archivo(ruta: str) -> Dict[int, Dict[str, Any]]:
    cuadros = {}
    try:
        with open(ruta, "r", encoding="utf-8", errors="replace") as f:
            for num_linea, linea in enumerate(f, 1):
                match = PATTERN.search(linea)
                if match:
                    rel = int(match.group(1))
                    comp = None
                    m_extra = EXTRA.search(linea)
                    if m_extra:
                        comp = {
                            "registros_por_linea": m_extra.group(1).lower(),
                            "VRAM": m_extra.group(2).lower(),
                            "OAM": m_extra.group(3).lower(),
                            "paleta": m_extra.group(4).lower(),
                            "paleta_lineas": m_extra.group(5).lower(),
                        }
                    cuadros[rel] = {
                        "rel": rel,
                        "frame": int(match.group(2)),
                        "ent": match.group(3).lower(),
                        "sal": match.group(4).lower(),
                        "bld0": match.group(5).lower(),
                        "bld50": match.group(6).lower(),
                        "bld100": match.group(7).lower(),
                        "bld150": match.group(8).lower(),
                        "bg3_blend": int(match.group(9)),
                        "comp": comp,
                        "linea": num_linea,
                    }
    except Exception as e:
        print(f"Error al leer '{ruta}': {e}", file=sys.stderr)
        sys.exit(2)
    return cuadros


def imprimir_resumen_salida(cuadros_android, cuadros_pc, todos_rels):
    total = len(todos_rels)
    if total == 0:
        return
    coincidentes = sum(
        1 for r in todos_rels
        if r in cuadros_android and r in cuadros_pc and cuadros_android[r]["sal"] == cuadros_pc[r]["sal"]
    )
    print(f"Resumen de salida GPU: {coincidentes}/{total} cuadros con salida idéntica ({coincidentes/total*100:.1f}%)")


def main():
    parser = argparse.ArgumentParser(
        description="Compara registros de hash de cuadros entre Android y PC."
    )
    parser.add_argument("--android", required=True, help="Ruta al log/archivo de Android")
    parser.add_argument("--pc", required=True, help="Ruta al log/archivo de PC")
    args = parser.parse_args()

    cuadros_android = parsear_archivo(args.android)
    cuadros_pc = parsear_archivo(args.pc)

    if not cuadros_android:
        print(f"Error: No se encontraron registros de hash validos en '{args.android}'", file=sys.stderr)
        sys.exit(2)

    if not cuadros_pc:
        print(f"Error: No se encontraron registros de hash validos en '{args.pc}'", file=sys.stderr)
        sys.exit(2)

    todos_rels = sorted(set(cuadros_android.keys()) | set(cuadros_pc.keys()))

    for rel in todos_rels:
        if rel not in cuadros_android:
            print(f"rel={rel}: Presente en PC pero ausente en Android", file=sys.stderr)
            sys.exit(1)
        if rel not in cuadros_pc:
            print(f"rel={rel}: Presente en Android pero ausente en PC", file=sys.stderr)
            sys.exit(1)

        ca = cuadros_android[rel]
        cp = cuadros_pc[rel]

        ambos_comp = ca.get("comp") is not None and cp.get("comp") is not None
        if ambos_comp:
            distintos = [k for k in ca["comp"] if ca["comp"][k] != cp["comp"][k]]
        else:
            distintos = []

        if ambos_comp and (ca["ent"] != cp["ent"] or distintos):
            if distintos:
                nombres = ", ".join(distintos)
                detalles = ", ".join(
                    f"{k} android={ca['comp'][k]} pc={cp['comp'][k]}" for k in distintos
                )
                print(
                    f"rel={rel}: ENTRADA difiere en: {nombres} ({detalles}) - salida no comparable"
                )
            else:
                print(
                    f"rel={rel}: ENTRADA difiere (android={ca['ent']}, pc={cp['ent']}, "
                    f"frame_android={ca['frame']}, frame_pc={cp['frame']})"
                )
            sys.exit(1)
        elif not ambos_comp and ca["ent"] != cp["ent"]:
            print(
                f"rel={rel}: ENTRADA difiere (android={ca['ent']}, pc={cp['ent']}, "
                f"frame_android={ca['frame']}, frame_pc={cp['frame']})"
            )
            sys.exit(1)

        if ca["sal"] != cp["sal"]:
            if ambos_comp:
                print(
                    f"rel={rel}: ENTRADAS iguales, SALIDA difiere ("
                    f"sal_android={ca['sal']}, sal_pc={cp['sal']}, "
                    f"frame_android={ca['frame']}, frame_pc={cp['frame']})"
                )
            else:
                print(
                    f"rel={rel}: ENTRADA igual, SALIDA difiere (ent={ca['ent']}, "
                    f"sal_android={ca['sal']}, sal_pc={cp['sal']}, "
                    f"frame_android={ca['frame']}, frame_pc={cp['frame']})"
                )
            imprimir_resumen_salida(cuadros_android, cuadros_pc, todos_rels)
            sys.exit(1)

        # Chequeo de campos secundarios de blender
        for k in ("bld0", "bld50", "bld100", "bld150", "bg3_blend"):
            if ca[k] != cp[k]:
                print(
                    f"rel={rel}: ENTRADA y SALIDA iguales, pero {k} difiere "
                    f"(android={ca[k]}, pc={cp[k]})"
                )
                sys.exit(1)

    imprimir_resumen_salida(cuadros_android, cuadros_pc, todos_rels)
    print(
        f"Exito: Coincidencia total en {len(todos_rels)} cuadros comparados (rel={todos_rels[0]}..{todos_rels[-1]})."
    )
    sys.exit(0)


if __name__ == "__main__":
    main()
