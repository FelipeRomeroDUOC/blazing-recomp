#!/usr/bin/env python3
"""
compilar_c_paralelo.py — Compilador multi-hilo de código C11 AOT para Android NDK.

Compila todas las unidades C generadas en src-c/ en paralelo aprovechando
todos los núcleos lógicos del procesador y las enlaza en libblazing_recompiled.so.
"""

import argparse
import concurrent.futures
import os
import subprocess
import sys
import time


def compilar_unidad(clang, triple, c_file, obj_file, inc_dir, opt_level):
    cmd = [
        clang,
        f"--target={triple}",
        opt_level,
        "-fPIC",
        "-fomit-frame-pointer",
        "-I",
        inc_dir,
        "-c",
        c_file,
        "-o",
        obj_file,
    ]
    t0 = time.time()
    res = subprocess.run(cmd, capture_output=True, text=True)
    dt = time.time() - t0
    if res.returncode != 0:
        return False, os.path.basename(c_file), dt, res.stderr
    return True, os.path.basename(c_file), dt, None


def main():
    parser = argparse.ArgumentParser(description="Compilador paralelo de unidades C recompiladas.")
    parser.add_argument("--clang", required=True, help="Ruta al binario clang del NDK.")
    parser.add_argument("--triple", required=True, help="Triple de destino (ej. aarch64-linux-android26).")
    parser.add_argument("--src-dir", required=True, help="Directorio src-c con los archivos C.")
    parser.add_argument("--out-dir", required=True, help="Directorio target/recompiled/<abi>.")
    parser.add_argument("--abi", required=True, help="Nombre del ABI (arm64-v8a, etc.).")
    parser.add_argument("--opt", default="-O1", help="Nivel de optimización (-O1, -O2, etc.).")
    parser.add_argument("--jobs", type=int, default=os.cpu_count() or 6, help="Hilos paralelos.")
    args = parser.parse_args()

    if not os.path.isdir(args.src_dir):
        print(f"Error: No existe el directorio '{args.src_dir}'", file=sys.stderr)
        sys.exit(1)

    os.makedirs(args.out_dir, exist_ok=True)

    header = os.path.join(args.src_dir, "recompiled.h")
    header_mtime = os.path.getmtime(header) if os.path.isfile(header) else 0.0

    # Buscar todas las unidades C
    c_files = sorted(
        [
            os.path.join(args.src_dir, f)
            for f in os.listdir(args.src_dir)
            if f.endswith(".c") and "-bios." in f
        ]
    )

    if not c_files:
        print(f"Aviso: No se encontraron archivos C en '{args.src_dir}'. Saltando.", file=sys.stderr)
        sys.exit(0)

    so_out = os.path.join(args.out_dir, "libblazing_recompiled.so")
    tareas = []
    obj_files = []

    for c_path in c_files:
        base = os.path.splitext(os.path.basename(c_path))[0]
        obj_path = os.path.join(args.out_dir, f"{base}.o")
        obj_files.append(obj_path)

        c_mtime = os.path.getmtime(c_path)
        need_compile = True
        if os.path.isfile(obj_path):
            obj_mtime = os.path.getmtime(obj_path)
            if obj_mtime > c_mtime and obj_mtime > header_mtime:
                need_compile = False

        if need_compile:
            tareas.append((c_path, obj_path))

    print(f"==> [{args.abi}] Compilando {len(tareas)} de {len(c_files)} unidades C en paralelo ({args.jobs} hilos, {args.opt})...")

    t_inicio = time.time()
    exitos = 0
    fallos = []

    if tareas:
        with concurrent.futures.ThreadPoolExecutor(max_workers=args.jobs) as executor:
            futuros = {
                executor.submit(
                    compilar_unidad,
                    args.clang,
                    args.triple,
                    c_path,
                    obj_path,
                    args.src_dir,
                    args.opt,
                ): c_path
                for c_path, obj_path in tareas
            }
            for fut in concurrent.futures.as_completed(futuros):
                ok, fname, dt, err = fut.result()
                if ok:
                    exitos += 1
                    print(f"   + [{args.abi}] {fname} -> .o ({dt:.1f}s) [{exitos}/{len(tareas)}]")
                else:
                    fallos.append((fname, err))
                    print(f"   [ERROR] Falló {fname}:\n{err}", file=sys.stderr)

    if fallos:
        print(f"Error: {len(fallos)} unidades fallaron en la compilación.", file=sys.stderr)
        sys.exit(1)

    # Enlazar biblioteca compartida
    need_link = True
    if os.path.isfile(so_out) and not tareas:
        need_link = False

    if need_link:
        print(f"   + [{args.abi}] Enlazando libblazing_recompiled.so ({len(obj_files)} unidades)...")
        link_cmd = [
            args.clang,
            f"--target={args.triple}",
            "-shared",
            args.opt,
            "-fomit-frame-pointer",
            "-Wl,-soname,libblazing_recompiled.so",
        ] + obj_files + ["-o", so_out]
        t_link = time.time()
        res_link = subprocess.run(link_cmd, capture_output=True, text=True)
        if res_link.returncode != 0:
            print(f"Error enlazando {so_out}:\n{res_link.stderr}", file=sys.stderr)
            sys.exit(res_link.returncode)
        so_size_mb = os.path.getsize(so_out) / (1024 * 1024)
        print(f"   + [{args.abi}] Generado {os.path.basename(so_out)} ({so_size_mb:.2f} MB en {time.time() - t_link:.1f}s)")
    else:
        print(f"   + [{args.abi}] libblazing_recompiled.so en caché (sin cambios)")

    print(f"[OK] [{args.abi}] Proceso completado en {time.time() - t_inicio:.1f}s.")


if __name__ == "__main__":
    main()
