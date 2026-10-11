#!/usr/bin/env python3
"""
analizar_telemetria.py — Analizador y comparador de registros de telemetría de Blazing Recomp.

Procesa la salida de métricas emitida por el motor en Android (Vulkan / GBA Core)
y genera un informe de rendimiento con percentiles de fotogramas, tiempos de GPU/CPU
y el ranking de los hotspots más demandantes del intérprete de emulación.
Incluye soporte para contrastar dos sesiones (--comparar) para verificar mejoras AOT.
"""

import argparse
import os
import re
import sys
from collections import Counter


def parsear_telemetria(ruta_o_stream):
    fps_muestras = []
    ejec_muestras = []
    ppu_muestras = []
    apu_muestras = []
    dibujo_muestras = []
    present_muestras = []
    interp_instrucciones = []
    nativas_instrucciones = []
    hotspots = Counter()

    lineas = ruta_o_stream.readlines() if hasattr(ruta_o_stream, "readlines") else []

    for linea in lineas:
        linea = linea.strip()
        if not linea:
            continue

        if linea.startswith("# hotspots top="):
            matches = re.findall(r"(0x[0-9A-Fa-f]+)\s*\((\d+)\)", linea)
            for addr, count in matches:
                hotspots[addr.upper()] += int(count)

        elif linea.startswith("t="):
            m = re.search(
                r"fps=([\d\.]+)\s+ejec=([\d\.]+)/[\d\.]+\s+ppu=([\d\.]+)\s+apu=([\d\.]+).*?"
                r"dibujo=([\d\.]+)/[\d\.]+.*?present=([\d\.]+)\s+nativos=(\d+)\s+interprete=(\d+)",
                linea,
            )
            if m:
                fps = float(m.group(1))
                ejec = float(m.group(2))
                ppu = float(m.group(3))
                apu = float(m.group(4))
                dibujo = float(m.group(5))
                present = float(m.group(6))
                nativos = int(m.group(7))
                interp = int(m.group(8))

                fps_muestras.append(fps)
                ejec_muestras.append(ejec)
                ppu_muestras.append(ppu)
                apu_muestras.append(apu)
                dibujo_muestras.append(dibujo)
                present_muestras.append(present)
                nativas_instrucciones.append(nativos)
                interp_instrucciones.append(interp)

    total_seg = len(fps_muestras)
    if total_seg == 0:
        return None

    fps_activos = [x for x in fps_muestras if x > 5.0]
    prom_fps = sum(fps_activos) / len(fps_activos) if fps_activos else 0.0
    min_fps = min(fps_activos) if fps_activos else 0.0
    max_fps = max(fps_activos) if fps_activos else 0.0

    prom_ejec = sum(ejec_muestras) / total_seg
    max_ejec = max(ejec_muestras) if ejec_muestras else 0.0
    prom_ppu = sum(ppu_muestras) / total_seg
    prom_apu = sum(apu_muestras) / total_seg
    prom_present = sum(present_muestras) / total_seg
    prom_dibujo = sum(dibujo_muestras) / total_seg

    total_nativas = sum(nativas_instrucciones)
    total_interp = sum(interp_instrucciones)
    total_total = total_nativas + total_interp
    ratio_aot = (total_nativas / total_total * 100.0) if total_total > 0 else 0.0

    return {
        "segundos": total_seg,
        "prom_fps": prom_fps,
        "min_fps": min_fps,
        "max_fps": max_fps,
        "prom_ejec": prom_ejec,
        "max_ejec": max_ejec,
        "prom_ppu": prom_ppu,
        "prom_apu": prom_apu,
        "prom_dibujo": prom_dibujo,
        "prom_present": prom_present,
        "total_nativas": total_nativas,
        "total_interp": total_interp,
        "ratio_aot": ratio_aot,
        "hotspots": hotspots,
    }


def imprimir_informe(d, top_n=20, titulo="INFORME DE TELEMETRÍA — BLAZING RECOMP"):
    print("=" * 72)
    print(f" {titulo}")
    print("=" * 72)
    print(f"Duración analizada:         {d['segundos']} segundos")
    print(f"Tasa de fotogramas (FPS):   {d['prom_fps']:.1f} FPS (mín: {d['min_fps']:.1f}, máx: {d['max_fps']:.1f})")
    print("-" * 72)
    print("DESGLOSE DE TIEMPOS POR FOTOGRAMA:")
    print(f"  • CPU Emulación (ejec):   {d['prom_ejec']:5.2f} ms  (peor segundo: {d['max_ejec']:.2f} ms)")
    print(f"  • PPU Software:           {d['prom_ppu']:5.2f} ms")
    print(f"  • APU Audio:              {d['prom_apu']:5.2f} ms")
    print(f"  • Render GPU (dibujo):    {d['prom_dibujo']:5.2f} ms")
    print(f"  • Presentación Swapchain: {d['prom_present']:5.2f} ms")
    print("-" * 72)
    print("COBERTURA DE EJECUCIÓN (AOT vs INTÉRPRETE):")
    print(f"  • Instrucciones Nativas AOT:  {d['total_nativas']:>12,} ({d['ratio_aot']:5.1f}%)")
    print(f"  • Instrucciones Intérprete:   {d['total_interp']:>12,} ({100.0 - d['ratio_aot']:5.1f}%)")
    print("=" * 72)

    hotspots = d["hotspots"]
    if hotspots:
        print(f"\nRANKING DE HOTSPOTS (TOP {top_n} DIRECCIONES EN EL INTÉRPRETE):")
        print(f"{'Dirección':<14} | {'Ticks':>10} | {'% Relativo':>10}")
        print("-" * 40)
        total_ticks = sum(hotspots.values())
        for addr, count in hotspots.most_common(top_n):
            pct = (count / total_ticks * 100.0) if total_ticks > 0 else 0.0
            print(f"{addr:<14} | {count:>10,} | {pct:>9.2f}%")
        print("=" * 72)


def comparar_sesiones(base, nuevo, nombre_base="Antes", nombre_nuevo="Nuevo"):
    print("=" * 72)
    print(" INFORME COMPARATIVO DE TELEMETRÍA — VERIFICACIÓN AOT")
    print("=" * 72)
    print(f"{'Métrica':<28} | {nombre_base:<15} | {nombre_nuevo:<15} | {'Diferencia':<12}")
    print("-" * 72)

    def diff_num(v_ant, v_nue, fmt="{:+.1f}", suf=""):
        delta = v_nue - v_ant
        return f"{fmt.format(delta)}{suf}"

    # FPS
    print(f"{'Tasa de FPS:':<28} | {base['prom_fps']:>5.1f} FPS       | {nuevo['prom_fps']:>5.1f} FPS       | {diff_num(base['prom_fps'], nuevo['prom_fps'], '{:+.1f}', ' FPS')}")
    # CPU ejec
    print(f"{'CPU Emulación (ejec):':<28} | {base['prom_ejec']:>5.2f} ms        | {nuevo['prom_ejec']:>5.2f} ms        | {diff_num(base['prom_ejec'], nuevo['prom_ejec'], '{:+.2f}', ' ms')}")
    # Render GPU
    print(f"{'Render GPU (dibujo):':<28} | {base['prom_dibujo']:>5.2f} ms        | {nuevo['prom_dibujo']:>5.2f} ms        | {diff_num(base['prom_dibujo'], nuevo['prom_dibujo'], '{:+.2f}', ' ms')}")
    # Swapchain
    print(f"{'Presentación Swapchain:':<28} | {base['prom_present']:>5.2f} ms        | {nuevo['prom_present']:>5.2f} ms        | {diff_num(base['prom_present'], nuevo['prom_present'], '{:+.2f}', ' ms')}")
    print("-" * 72)
    # Cobertura AOT
    print(f"{'Cobertura Nativa AOT:':<28} | {base['ratio_aot']:>5.1f} %        | {nuevo['ratio_aot']:>5.1f} %        | {diff_num(base['ratio_aot'], nuevo['ratio_aot'], '{:+.1f}', ' %')}")
    # Instrucciones intérprete
    red_interp = ((nuevo['total_interp'] - base['total_interp']) / base['total_interp'] * 100.0) if base['total_interp'] > 0 else 0.0
    print(f"{'Instrucciones Intérprete:':<28} | {base['total_interp']:>10,}     | {nuevo['total_interp']:>10,}     | {red_interp:+.1f} %")
    print("=" * 72)

    # Estado de los top hotspots del log base en el nuevo log
    print(f"\nESTADO DE LOS TOP HOTSPOTS DE '{nombre_base}' EN '{nombre_nuevo}':")
    print(f"{'Dirección':<14} | {'Ticks ' + nombre_base:>12} | {'Ticks ' + nombre_nuevo:>12} | {'Reducción':<12}")
    print("-" * 56)
    for addr, cnt_base in base["hotspots"].most_common(10):
        cnt_nuevo = nuevo["hotspots"].get(addr, 0)
        red_pct = ((cnt_nuevo - cnt_base) / cnt_base * 100.0) if cnt_base > 0 else 0.0
        estado = f"{red_pct:+.1f} %" if cnt_nuevo > 0 else "ELIMINADO (100% NATIVO)"
        print(f"{addr:<14} | {cnt_base:>12,} | {cnt_nuevo:>12,} | {estado}")
    print("=" * 72)


def main():
    parser = argparse.ArgumentParser(
        description="Analiza y compara archivos de telemetría de rendimiento de Blazing Recomp."
    )
    parser.add_argument(
        "--archivo",
        "-f",
        type=str,
        default=None,
        help="Ruta al archivo de log de telemetría (si se omite, lee de stdin).",
    )
    parser.add_argument(
        "--comparar",
        "-c",
        type=str,
        default=None,
        help="Ruta al archivo base para comparar (genera informe comparativo Antes vs Después).",
    )
    parser.add_argument(
        "--top",
        "-t",
        type=int,
        default=20,
        help="Número de hotspots a mostrar en el ranking (defecto: 20).",
    )
    args = parser.parse_args()

    if args.comparar:
        if not args.archivo:
            print("Error: Para usar --comparar debes especificar también --archivo <nuevo_log>.", file=sys.stderr)
            sys.exit(1)
        if not os.path.isfile(args.comparar):
            print(f"Error: No se encontró el archivo base '{args.comparar}'.", file=sys.stderr)
            sys.exit(1)
        if not os.path.isfile(args.archivo):
            print(f"Error: No se encontró el archivo nuevo '{args.archivo}'.", file=sys.stderr)
            sys.exit(1)

        with open(args.comparar, "r", encoding="utf-8", errors="replace") as fb:
            datos_base = parsear_telemetria(fb)
        with open(args.archivo, "r", encoding="utf-8", errors="replace") as fn:
            datos_nuevo = parsear_telemetria(fn)

        if not datos_base or not datos_nuevo:
            print("Error: No se pudieron extraer muestras válidas de ambos archivos.", file=sys.stderr)
            sys.exit(1)

        comparar_sesiones(
            datos_base,
            datos_nuevo,
            nombre_base=os.path.basename(args.comparar),
            nombre_nuevo=os.path.basename(args.archivo),
        )
        return

    if args.archivo:
        if not os.path.isfile(args.archivo):
            print(f"Error: No se encontró el archivo '{args.archivo}'.", file=sys.stderr)
            sys.exit(1)
        with open(args.archivo, "r", encoding="utf-8", errors="replace") as f:
            d = parsear_telemetria(f)
            if d:
                imprimir_informe(d, top_n=args.top)
            else:
                print("Aviso: No se encontraron muestras de fotogramas ('t=Xs fps=...') en la entrada.")
    else:
        if sys.stdin.isatty():
            parser.print_help()
            sys.exit(1)
        d = parsear_telemetria(sys.stdin)
        if d:
            imprimir_informe(d, top_n=args.top)
        else:
            print("Aviso: No se encontraron muestras de fotogramas ('t=Xs fps=...') en la entrada.")


if __name__ == "__main__":
    main()
