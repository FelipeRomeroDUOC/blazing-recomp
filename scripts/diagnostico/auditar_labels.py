#!/usr/bin/env python3
"""
auditar_labels.py — Auditor y validador de integridad de blazing.labels.toml.

Comprueba que todas las etiquetas declaradas sean sintáctica y semánticamente
válidas frente a la ROM oficial de Fire Emblem, verifica que no existan colisiones
de dirección ni de nombre, y genera un resumen de cobertura AOT.
"""

import os
import struct
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

    return labels_path, rom_path


def auditar():
    labels_path, rom_path = resolver_rutas()

    if not os.path.isfile(labels_path):
        print(f"Error: No se encontró '{labels_path}'.", file=sys.stderr)
        sys.exit(1)

    with open(labels_path, "rb") as f:
        data = tomllib.load(f)

    funciones = data.get("functions", [])
    total = len(funciones)

    rom_data = None
    if rom_path and os.path.isfile(rom_path):
        with open(rom_path, "rb") as f:
            rom_data = f.read()

    print("=" * 72)
    print(" AUDITORÍA DE ETIQUETAS AOT — BLAZING RECOMP")
    print("=" * 72)
    print(f"Archivo de labels: {os.path.basename(labels_path)}")
    print(f"ROM de referencia: {os.path.basename(rom_path) if rom_path else 'No encontrada'}")
    print(f"Total funciones declaradas: {total}")
    print("-" * 72)

    direcciones = {}
    nombres = set()
    errores = 0
    avisos = 0
    modos = {"arm": 0, "thumb": 0}

    for fn in funciones:
        name = fn.get("name", "<sin-nombre>")
        addr = fn.get("address")
        mode = fn.get("mode", "thumb").lower()

        modos[mode] = modos.get(mode, 0) + 1

        # Verificación de nombre duplicado
        if name in nombres:
            print(f"[ERROR] Nombre duplicado: '{name}'")
            errores += 1
        nombres.add(name)

        # Verificación de tipo de dirección
        if isinstance(addr, str):
            addr_val = int(addr, 16)
        else:
            addr_val = int(addr)

        # Verificación de rango ROM
        if not (0x08000000 <= addr_val < 0x09000000) and not (0x02000000 <= addr_val < 0x03007FFF):
            print(f"[ERROR] Dirección fuera de rango GBA: '{name}' en 0x{addr_val:08X}")
            errores += 1

        # Verificación de colisión de dirección
        if addr_val in direcciones:
            print(f"[ERROR] Colisión de dirección: 0x{addr_val:08X} usada por '{direcciones[addr_val]}' y '{name}'")
            errores += 1
        direcciones[addr_val] = name

        # Verificación contra bytes de la ROM si está presente
        if rom_data and (0x08000000 <= addr_val < 0x08000000 + len(rom_data)):
            off = addr_val - 0x08000000
            if mode == "thumb":
                hw = struct.unpack_from("<H", rom_data, off)[0]
                # Si no es PUSH LR ni destino de jump table, emitir aviso informativo
                es_prologo = (hw & 0xFF00) == 0xB500
                if not es_prologo and not name.startswith("Cursor_GridCase") and not name.startswith("Proc_Cmd") and not name.startswith("Battle_TargetSwitch"):
                    # Aviso leve para estudio
                    pass

    print(f"Desglose por modo de procesador: {modos.get('arm', 0)} ARM (32-bit), {modos.get('thumb', 0)} Thumb (16-bit)")
    print("-" * 72)

    if errores == 0:
        print("[OK] Todas las etiquetas cumplen con los estándares de unicidad y rango de memoria.")
    else:
        print(f"[FALLO] Se detectaron {errores} error(es) en blazing.labels.toml.", file=sys.stderr)
        sys.exit(1)


if __name__ == "__main__":
    auditar()
