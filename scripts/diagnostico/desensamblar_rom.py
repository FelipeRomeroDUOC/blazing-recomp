#!/usr/bin/env python3
"""
desensamblar_rom.py — Desensamblador Thumb y buscador de funciones en la ROM de Fire Emblem.

Inspecciona la ROM oficial de GBA para decodificar instrucciones en modo Thumb (16 bits),
identificar prólogos canónicos de funciones bajo la ABI de ARM (PUSH {..., lr}) y
analizar vectores de tablas de salto indirectas (mov pc, rx).
"""

import argparse
import os
import struct
import sys


def resolver_rom(ruta_provista=None):
    if ruta_provista and os.path.isfile(ruta_provista):
        return ruta_provista

    # Búsqueda automática en roms/
    repo_root = os.path.dirname(os.path.dirname(os.path.dirname(os.path.abspath(__file__))))
    roms_dir = os.path.join(repo_root, "roms")
    if os.path.isdir(roms_dir):
        for f in os.listdir(roms_dir):
            if f.lower().endswith(".gba"):
                return os.path.join(roms_dir, f)

    return None


def decodificar_thumb(hw, hw2=None):
    """Decodifica opcodes Thumb comunes para diagnóstico."""
    if hw == 0x4770:
        return "bx lr", "RETORNO DE FUNCIÓN"
    if (hw & 0xFF00) == 0xB500:
        regs = []
        for bit in range(8):
            if hw & (1 << bit):
                regs.append(f"r{bit}")
        if hw & 0x0100:
            regs.append("lr")
        return f"push {{{', '.join(regs)}}}", "PRÓLOGO DE SUBRUTINA (ABI)"
    if (hw & 0xFF00) == 0xBD00:
        regs = []
        for bit in range(8):
            if hw & (1 << bit):
                regs.append(f"r{bit}")
        if hw & 0x0100:
            regs.append("pc")
        return f"pop {{{', '.join(regs)}}}", "EPÍLOGO Y RETORNO"
    if (hw & 0xFF87) == 0x4687:
        rm = (hw >> 3) & 0x0F
        return f"mov pc, r{rm}", "SALTO INDIRECTO / JUMP TABLE"
    if (hw & 0xF800) == 0xF000 and hw2 is not None and (hw2 & 0xF800) == 0xF800:
        return "bl <subrutina>", "LLAMADA A FUNCIÓN (BL PAIR)"
    if (hw & 0xF800) == 0xE000:
        offset = hw & 0x07FF
        if offset & 0x0400:
            offset -= 0x0800
        return f"b <offset {offset * 2:+d}>", "SALTO INCONDICIONAL"

    return f"0x{hw:04X}", ""


def desensamblar(rom_data, gba_addr, count=30):
    offset = (gba_addr - 0x08000000) & ~1
    if offset < 0 or offset >= len(rom_data):
        print(f"Error: Dirección 0x{gba_addr:08X} fuera de los límites de la ROM.")
        return

    print("=" * 72)
    print(f" DESENSAMBLADO THUMB — BASE: 0x{gba_addr:08X} (Offset: 0x{offset:06X})")
    print("=" * 72)
    print(f"{'Dirección GBA':<14} | {'Hex':<8} | {'Instrucción':<24} | {'Anotación':<20}")
    print("-" * 72)

    i = 0
    while i < count and (offset + i * 2) < len(rom_data):
        cur_off = offset + i * 2
        cur_addr = 0x08000000 + cur_off
        hw = struct.unpack_from("<H", rom_data, cur_off)[0]
        hw2 = None
        if (cur_off + 2) < len(rom_data):
            hw2 = struct.unpack_from("<H", rom_data, cur_off + 2)[0]

        inst, nota = decodificar_thumb(hw, hw2)
        print(f"0x{cur_addr:08X}   | 0x{hw:04X}   | {inst:<24} | {nota}")
        i += 1


def buscar_prologo(rom_data, target_addr, scan_bytes=1024):
    target_off = (target_addr - 0x08000000) & ~1
    scan_start = max(0, target_off - scan_bytes)

    prologos = []
    for off in range(scan_start, target_off + 2, 2):
        hw = struct.unpack_from("<H", rom_data, off)[0]
        if (hw & 0xFF00) == 0xB500:  # PUSH {..., lr}
            addr = 0x08000000 + off
            dist = target_off - off
            inst, _ = decodificar_thumb(hw)
            prologos.append((addr, inst, dist))

    if not prologos:
        print(f"No se detectó ningún prólogo PUSH {{..., lr}} en los {scan_bytes} bytes previos a 0x{target_addr:08X}.")
        return

    print("=" * 72)
    print(f" BÚSQUEDA DE PRÓLOGOS ABI ANTERIORES A: 0x{target_addr:08X}")
    print("=" * 72)
    for addr, inst, dist in prologos:
        es_inmediato = " <== CANDIDATO INMEDIATO" if addr == prologos[-1][0] else ""
        print(f"  • 0x{addr:08X}: {inst:<24} (distancia: -{dist} bytes){es_inmediato}")
    print("=" * 72)


def main():
    parser = argparse.ArgumentParser(
        description="Herramienta de desensamblado Thumb e inspección de funciones en la ROM de GBA."
    )
    parser.add_argument("--rom", "-r", type=str, default=None, help="Ruta al archivo .gba oficial.")
    parser.add_argument(
        "--addr",
        "-a",
        type=lambda x: int(x, 16),
        default=None,
        help="Dirección hexadecimal GBA a desensamblar (ej. 0x08013280).",
    )
    parser.add_argument(
        "--count",
        "-c",
        type=int,
        default=30,
        help="Número de instrucciones a desensamblar (defecto: 30).",
    )
    parser.add_argument(
        "--buscar-prologo",
        "-p",
        type=lambda x: int(x, 16),
        default=None,
        help="Busca el prólogo de subrutina (PUSH LR) anterior a la dirección dada.",
    )

    args = parser.parse_args()

    rom_path = resolver_rom(args.rom)
    if not rom_path:
        print("Error: No se encontró la ROM de Fire Emblem en roms/ ni se especificó --rom.", file=sys.stderr)
        sys.exit(1)

    with open(rom_path, "rb") as f:
        rom_data = f.read()

    if args.buscar_prologo:
        buscar_prologo(rom_data, args.buscar_prologo)
    elif args.addr:
        desensamblar(rom_data, args.addr, count=args.count)
    else:
        parser.print_help()


if __name__ == "__main__":
    main()
