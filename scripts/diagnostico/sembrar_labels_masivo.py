#!/usr/bin/env python3
"""
sembrar_labels_masivo.py — Sembrador masivo y automatizado de etiquetas AOT.

Escanea la ROM oficial de Fire Emblem buscando todas las tablas de punteros
de función de la arquitectura ProcEngine (corrutinas, callbacks, dispatchers)
y genera automáticamente las entradas en blazing.labels.toml, preservando
todas las etiquetas manuales previamente definidas.
"""

import argparse
import os
import sys
import tomllib


def resolver_rutas():
    repo_root = os.path.dirname(os.path.dirname(os.path.dirname(os.path.abspath(__file__))))
    labels_path = os.path.join(repo_root, "blazing.labels.toml")
    roms_dir = os.path.join(repo_root, "roms")
    rom_path = None
    if os.path.isdir(roms_dir):
        for f in os.listdir(roms_dir):
            if f.lower().endswith(".gba"):
                rom_path = os.path.join(roms_dir, f)
                break
    return repo_root, labels_path, rom_path


def escanear_punteros(rom_bytes, solo_tablas=True):
    """
    Escanea la ROM en busca de funciones Thumb válidas.
    Si solo_tablas=True, extrae solo las que son referenciadas por tablas de punteros.
    Si solo_tablas=False, extrae todas las subrutinas con prólogo ABI en la sección de código.
    """
    candidatos = set()
    code_start = 0x080000C0
    code_end = 0x08150000  # Sección de código ejecutable de FE7

    if solo_tablas:
        # Escaneo de palabras de 32 bits alineadas a 4 bytes
        for i in range(0, len(rom_bytes) - 4, 4):
            word = int.from_bytes(rom_bytes[i:i+4], "little")
            if code_start < word < code_end and (word & 1) == 1:
                addr = word & ~1
                off = addr - 0x08000000
                if 0 <= off < len(rom_bytes) - 2:
                    op = int.from_bytes(rom_bytes[off:off+2], "little")
                    # Prólogo ABI: PUSH {..., lr} (0xB5xx) o PUSH {..., r4-r7} (0xB4xx)
                    if (op & 0xFF00) in (0xB500, 0xB400):
                        candidatos.add(addr)
    else:
        # Unión de punteros de tablas + escaneo de todo prólogo B5xx en código
        cand_tablas = escanear_punteros(rom_bytes, solo_tablas=True)
        candidatos.update(cand_tablas)
        max_off = min(code_end - 0x08000000, len(rom_bytes) - 2)
        for off in range(code_start - 0x08000000, max_off, 2):
            op = int.from_bytes(rom_bytes[off:off+2], "little")
            if (op & 0xFF00) == 0xB500:
                candidatos.add(0x08000000 + off)

    return sorted(list(candidatos))


def main():
    parser = argparse.ArgumentParser(description="Sembrado masivo de labels AOT desde la ROM.")
    parser.add_argument("--aplicar", action="store_true", help="Sobrescribe blazing.labels.toml con las nuevas etiquetas.")
    parser.add_argument("--exhaustivo", action="store_true", help="Escanea todos los prólogos ABI del código, no solo los de tablas.")
    parser.add_argument("--max", type=int, default=0, help="Límite máximo de nuevas etiquetas a incorporar (0 = sin límite).")
    args = parser.parse_args()

    repo_root, labels_path, rom_path = resolver_rutas()

    if not rom_path or not os.path.isfile(rom_path):
        print(f"Error: No se encontró la ROM en '{repo_root}/roms'.", file=sys.stderr)
        sys.exit(1)

    with open(rom_path, "rb") as f:
        rom_bytes = f.read()

    with open(labels_path, "r", encoding="utf-8") as f:
        contenido_original = f.read()

    # Base manual pura (sin bloques autogenerados previos)
    marcador = "# --- FUNCIONES AUTOGENERADAS POR SEMBRADO MASIVO ---"
    if marcador in contenido_original:
        contenido_base = contenido_original.split(marcador)[0].rstrip()
    else:
        contenido_base = contenido_original.rstrip()

    labels_data = tomllib.loads(contenido_base)
    funciones_base = labels_data.get("functions", [])
    addrs_base = set()
    for fn in funciones_base:
        a = fn["address"]
        val = int(a) if not isinstance(a, str) else int(a, 16)
        addrs_base.add(val)

    print("=" * 72)
    print(" SEMBRADO MASIVO DE ETIQUETAS AOT — BLAZING RECOMP")
    print("=" * 72)
    print(f"ROM objetivo: {os.path.basename(rom_path)}")
    print(f"Etiquetas manuales base: {len(funciones_base)}")
    modo_str = "Exhaustivo (todo prólogo ABI + tablas)" if args.exhaustivo else "Tablas de punteros (ProcEngine / callbacks)"
    print(f"Modo de escaneo: {modo_str}")
    print("-" * 72)

    candidatos = escanear_punteros(rom_bytes, solo_tablas=not args.exhaustivo)
    nuevas = [a for a in candidatos if a not in addrs_base]

    if args.max > 0:
        nuevas = nuevas[:args.max]

    print(f"Candidatos detectados en ROM: {len(candidatos)}")
    print(f"Nuevas etiquetas a incorporar: {len(nuevas)}")
    print(f"Total resultante de etiquetas: {len(funciones_base) + len(nuevas)}")
    print("-" * 72)

    if not args.aplicar:
        print("[INFO] Modo simulación. Para escribir los cambios, ejecuta con --aplicar.")
        sys.exit(0)

    bloques_nuevos = [marcador, "# Total añadidas en esta tanda: " + str(len(nuevas)), ""]
    for a in nuevas:
        bloques_nuevos.append("[[functions]]")
        bloques_nuevos.append(f'name = "FE7_Proc_{a:08X}"')
        bloques_nuevos.append(f"address = 0x{a:08x}")
        bloques_nuevos.append('mode = "thumb"')
        bloques_nuevos.append("")

    nuevo_contenido = contenido_base + "\n\n" + "\n".join(bloques_nuevos) + "\n"

    with open(labels_path, "w", encoding="utf-8") as f:
        f.write(nuevo_contenido)

    print(f"[OK] Se actualizaron exitosamente {len(nuevas)} nuevas etiquetas en '{os.path.basename(labels_path)}'.")


if __name__ == "__main__":
    main()
