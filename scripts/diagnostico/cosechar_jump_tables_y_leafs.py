#!/usr/bin/env python3
"""
scripts/diagnostico/cosechar_jump_tables_y_leafs.py
Cosecha automatizada de destinos de Jump Tables (mov pc, rx), funciones hoja (leaf functions)
y subrutinas ARM en la ROM de Fire Emblem (Europe) para cobertura nativa AOT completa.
"""

import sys
import struct
import re
from pathlib import Path

ROM_PATH = Path("roms/1575 - Fire Emblem (E)(Independent).gba")
LABELS_PATH = Path("blazing.labels.toml")

ROM_BASE = 0x08000000
ROM_MAX = 0x09000000  # 16 MB

def read16(rom: bytes, off: int) -> int:
    return rom[off] | (rom[off + 1] << 8)

def read32(rom: bytes, off: int) -> int:
    return struct.unpack("<I", rom[off:off + 4])[0]

def es_direccion_rom_valida(addr: int, rom_len: int) -> bool:
    if addr < ROM_BASE or addr >= ROM_BASE + rom_len:
        return False
    return (addr % 2) == 0

def cosechar_jump_tables(rom: bytes) -> dict:
    """
    Escanea instrucciones 'mov pc, rx' en Thumb, localiza sus tablas de punteros
    y extrae los destinos correspondientes a los casos de switch.
    """
    rom_len = len(rom)
    destinos = {} # addr -> nombre
    total_jts = 0

    for off in range(0, min(rom_len, 0x01000000) - 2, 2):
        half = read16(rom, off)
        # mov pc, rx: 0x4687, 0x468F, ..., 0x46FF (high register mov a r15)
        if (half & 0xFF87) == 0x4687:
            reg_src = (half >> 3) & 0xF
            addr_jt = ROM_BASE + off
            total_jts += 1

            # 1. Buscar cmp reg, #count hacia atras (hasta 40 bytes / 20 instrucciones)
            count = None
            for back in range(2, 42, 2):
                if off - back < 0:
                    break
                w = read16(rom, off - back)
                if (w & 0xF800) == 0x2800: # cmp reg, #imm8
                    count = (w & 0xFF) + 1
                    break

            if not count or count > 256:
                count = 32 # Limite defensivo para tablas sin cmp inmediato detectado

            # 2. Localizar la referencia a la tabla de punteros (ldr reg, [pc, #imm])
            tbl_addr = None
            for back in range(2, 42, 2):
                if off - back < 0:
                    break
                w = read16(rom, off - back)
                if (w & 0xF800) == 0x4800: # ldr reg, [pc, #imm]
                    imm = (w & 0xFF) * 4
                    pc_aligned = ((addr_jt - back + 4) & ~3) - ROM_BASE
                    if 0 <= pc_aligned + imm + 4 <= rom_len:
                        ptr_candidato = read32(rom, pc_aligned + imm)
                        if es_direccion_rom_valida(ptr_candidato, rom_len):
                            tbl_addr = ptr_candidato
                            break

            # 3. Si no hay ldr pc directo, verificar si la tabla es contigua al salto alineada a 4 bytes
            if not tbl_addr:
                inline_off = (off + 2 + 3) & ~3
                if inline_off + 4 <= rom_len:
                    primero = read32(rom, inline_off)
                    if es_direccion_rom_valida(primero, rom_len):
                        tbl_addr = ROM_BASE + inline_off

            # 4. Extraer entradas de la tabla
            if tbl_addr:
                tbl_off = tbl_addr - ROM_BASE
                consecutivos_validos = 0
                for i in range(count):
                    entrada_off = tbl_off + (i * 4)
                    if entrada_off + 4 > rom_len:
                        break
                    raw_target = read32(rom, entrada_off)
                    target = raw_target & ~1
                    if es_direccion_rom_valida(target, rom_len):
                        # Descartar palabras 0x00000000 o 0xFFFFFFFF
                        t_off = target - ROM_BASE
                        w_t = read16(rom, t_off)
                        if w_t != 0x0000 and w_t != 0xFFFF:
                            destinos[target] = f"FE7_JT_08{off:06X}_case_{i:02X}"
                            consecutivos_validos += 1
                        else:
                            break
                    else:
                        break

    return destinos

def cosechar_punteros_funciones(rom: bytes) -> dict:
    """
    Escanea todas las palabras de 32 bits de la ROM con bit 0 en 1 (punteros de funcion Thumb).
    Identifica stubs de callback, leaf functions y funciones registradas en estructuras de datos.
    """
    rom_len = len(rom)
    destinos = {}

    for off in range(0, min(rom_len, 0x01000000) - 4, 4):
        val = read32(rom, off)
        # Formato canónico de puntero a función Thumb: 0x08xxxxxx con bit 0 = 1
        if (val & 0xFF000001) == 0x08000001:
            target = val & ~1
            if es_direccion_rom_valida(target, rom_len):
                t_off = target - ROM_BASE
                w = read16(rom, t_off)
                # Debe contener al menos una instruccion Thumb valida no-cero
                if w != 0x0000 and w != 0xFFFF:
                    # Comprobar que no sea un bloque de datos puros buscando retorno o instruccion ejecutable
                    # en las primeras 8 instrucciones
                    tiene_retorno_o_logica = False
                    for step in range(0, 256, 2):
                        if t_off + step + 2 > rom_len:
                            break
                        inst = read16(rom, t_off + step)
                        if inst == 0x4770 or (inst & 0xFF00) == 0xBD00: # bx lr o pop {..., pc}
                            tiene_retorno_o_logica = True
                            break
                        if (inst & 0xFF00) == 0xB500: # push {..., lr}
                            tiene_retorno_o_logica = True
                            break
                    if tiene_retorno_o_logica:
                        destinos[target] = f"FE7_FnPtr_08{target - ROM_BASE:06X}"

    return destinos

def cosechar_rutinas_m4a(rom: bytes) -> dict:
    """
    Escanea funciones de audio de la libreria M4A/MP2K que preservan el registro de enlace
    con 'mov r12, lr' (0x46F4) y retornan con 'bx r12' (0x4760).
    """
    rom_len = len(rom)
    m4a_funcs = {}
    for off in range(0, min(rom_len, 0x01000000) - 4, 2):
        half = rom[off] | (rom[off + 1] << 8)
        if half == 0x46F4:  # mov r12, lr
            # Verificar si en los siguientes 256 bytes hay retorno bx r12
            for step in range(2, 256, 2):
                if off + step + 2 > rom_len:
                    break
                h2 = rom[off + step] | (rom[off + step + 1] << 8)
                if h2 == 0x4760:  # bx r12
                    addr = ROM_BASE + off
                    m4a_funcs[addr] = f"FE7_M4A_Driver_08{off:06X}"
                    break
    return m4a_funcs

def obtener_trampolines_thumb() -> dict:
    """
    Registra los puntos de entrada Thumb de los trampolines hacia modo ARM.
    """
    return {
        0x080C635C: "FE7_ThmTrampoline_080C635C",
        0x080C6364: "FE7_ThmTrampoline_080C6364",
        0x080C636C: "FE7_ThmTrampoline_080C636C",
        0x080C6374: "FE7_ThmTrampoline_080C6374",
        0x080C637C: "FE7_ThmTrampoline_080C637C",
        0x080C6384: "FE7_ThmTrampoline_080C6384",
    }

def obtener_rutinas_arm_criticas() -> dict:
    """
    Registra las subrutinas en modo ARM de 32 bits identificadas en perfiles de telemetria,
    incluyendo controladores FastMem, vectores del sistema y trampolines ARM.
    """
    arm_funcs = {
        # Rutinas de manipulacion rapida de memoria (FastMem)
        0x080BE404: "FE7_FastMem_ARM_080BE404",
        0x080BE408: "FE7_FastMem_ARM_080BE408",
        0x080BE40C: "FE7_FastMem_ARM_080BE40C",

        # Rutinas y servicios ARM en la cabecera de la ROM
        0x08000200: "FE7_System_ARM_08000200",
        0x08000234: "FE7_System_ARM_08000234",
        0x08000304: "FE7_System_ARM_08000304",
        0x08000360: "FE7_System_ARM_08000360",
        0x080003A8: "FE7_System_ARM_080003A8",
        0x080003E0: "FE7_System_ARM_080003E0",
        0x0800043C: "FE7_System_ARM_0800043C",
        0x08000494: "FE7_System_ARM_08000494",

        # Destinos ARM de los trampolines de conmutacion bx pc
        0x080C6360: "FE7_ArmTrampoline_080C6360",
        0x080C6368: "FE7_ArmTrampoline_080C6368",
        0x080C6370: "FE7_ArmTrampoline_080C6370",
        0x080C6378: "FE7_ArmTrampoline_080C6378",
        0x080C6380: "FE7_ArmTrampoline_080C6380",
        0x080C6388: "FE7_ArmTrampoline_080C6388",
        0x080C638C: "FE7_ArmTrampoline_080C638C",
    }
    return arm_funcs

def obtener_subrutinas_criticas_telemetria() -> dict:
    """
    Subrutinas y bucles identificados directamente en los perfiles de telemetria
    de Android TV (Run 12) para garantizar su incorporacion incondicional.
    """
    return {
        0x0800A502: "FE7_Loop_0800A502",
        0x080841A0: "FE7_Loop_080841A0",
        0x080BE900: "FE7_Leaf_080BE900",
        0x080AA9AC: "FE7_Leaf_080AA9AC",
    }

def main():
    if not ROM_PATH.exists():
        print(f"ERROR: No se encontró la ROM en '{ROM_PATH}'.")
        sys.exit(1)

    if not LABELS_PATH.exists():
        print(f"ERROR: No se encontró el archivo de labels en '{LABELS_PATH}'.")
        sys.exit(1)

    print(f"==> Leyendo ROM oficial: {ROM_PATH}")
    with open(ROM_PATH, "rb") as f:
        rom = f.read()

    print("==> 1. Cosechando destinos de Jump Tables (mov pc, rx)...")
    jt_targets = cosechar_jump_tables(rom)
    print(f"    Destinos de Jump Table identificados: {len(jt_targets)}")

    print("==> 2. Cosechando punteros de funcion Thumb y Leaf Functions...")
    fn_ptrs = cosechar_punteros_funciones(rom)
    print(f"    Punteros de funcion validos: {len(fn_ptrs)}")

    print("==> 3. Cosechando funciones de audio M4A (mov r12, lr / bx r12)...")
    m4a_funcs = cosechar_rutinas_m4a(rom)
    print(f"    Funciones M4A identificadas: {len(m4a_funcs)}")

    print("==> 4. Cosechando trampolines Thumb hacia ARM...")
    thm_trampolines = obtener_trampolines_thumb()
    print(f"    Trampolines Thumb identificados: {len(thm_trampolines)}")

    print("==> 5. Cosechando subrutinas en modo ARM de 32 bits...")
    arm_funcs = obtener_rutinas_arm_criticas()
    print(f"    Subrutinas ARM identificadas: {len(arm_funcs)}")

    print("==> 6. Cosechando hotspots criticos de telemetria (Android TV Run 12)...")
    subrutinas_telemetria = obtener_subrutinas_criticas_telemetria()
    print(f"    Hotspots criticos de telemetria: {len(subrutinas_telemetria)}")

    # Cargar labels existentes
    print(f"==> 7. Cargando labels existentes desde '{LABELS_PATH}'...")
    with open(LABELS_PATH, "r", encoding="utf-8") as f:
        content = f.read()

    # Parsear direcciones existentes
    existentes = set()
    for m in re.finditer(r"address\s*=\s*(0x[0-9a-fA-F]+)", content):
        existentes.add(int(m.group(1), 16))

    print(f"    Labels existentes: {len(existentes)}")

    # Unificar candidatos sin colisiones
    nuevas_thumb = {}
    for addr, name in subrutinas_telemetria.items():
        if addr not in existentes and addr not in arm_funcs:
            nuevas_thumb[addr] = name

    for addr, name in jt_targets.items():
        if addr not in existentes and addr not in nuevas_thumb and addr not in arm_funcs:
            nuevas_thumb[addr] = name

    for addr, name in fn_ptrs.items():
        if addr not in existentes and addr not in nuevas_thumb and addr not in arm_funcs:
            nuevas_thumb[addr] = name

    for addr, name in m4a_funcs.items():
        if addr not in existentes and addr not in nuevas_thumb and addr not in arm_funcs:
            nuevas_thumb[addr] = name

    for addr, name in thm_trampolines.items():
        if addr not in existentes and addr not in nuevas_thumb and addr not in arm_funcs:
            nuevas_thumb[addr] = name

    nuevas_arm = {}
    for addr, name in arm_funcs.items():
        if addr not in existentes:
            nuevas_arm[addr] = name

    print(f"==> Nuevas labels a incorporar:")
    print(f"    Thumb (Jump Tables + Leaf + M4A + Trampolines): {len(nuevas_thumb)}")
    print(f"    ARM 32-bit (System + Trampolines + FastMem):     {len(nuevas_arm)}")

    if not nuevas_thumb and not nuevas_arm:
        print("==> No hay nuevas labels para incorporar. Todo ya esta mapeado.")
        return

    # Generar bloques toml para agregar
    lineas = []
    lineas.append("\n# ==============================================================================")
    lineas.append("# DESTINOS DE JUMP TABLES, LEAF FUNCTIONS Y SUBRUTINAS ARM COSECHADAS")
    lineas.append("# ==============================================================================\n")

    # Ordenar por direccion
    for addr in sorted(nuevas_thumb.keys()):
        name = nuevas_thumb[addr]
        lineas.append("[[functions]]")
        lineas.append(f'name = "{name}"')
        lineas.append(f"address = 0x{addr:08x}")
        lineas.append('mode = "thumb"\n')

    for addr in sorted(nuevas_arm.keys()):
        name = nuevas_arm[addr]
        lineas.append("[[functions]]")
        lineas.append(f'name = "{name}"')
        lineas.append(f"address = 0x{addr:08x}")
        lineas.append('mode = "arm"\n')

    nuevo_bloque = "\n".join(lineas)

    with open(LABELS_PATH, "a", encoding="utf-8") as f:
        f.write(nuevo_bloque)

    total_final = len(existentes) + len(nuevas_thumb) + len(nuevas_arm)
    print(f"==> Exito: blazing.labels.toml actualizado con {total_final} labels totales.")

if __name__ == "__main__":
    main()
