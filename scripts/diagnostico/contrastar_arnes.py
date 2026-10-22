#!/usr/bin/env python3
"""Captura y contrasta la salida de los experimentos de `examples/`.

Sirve para lo que se hizo a mano al consolidar el arnés común: correr unos cuantos
experimentos antes y después de tocarlo y comprobar que **dicen lo mismo**. La
comparación es byte a byte, de la salida por pantalla y de los volcados que escriben.

Dos subcomandos:

    capturar <directorio>     construye los experimentos y guarda su salida y volcados
    comparar <a> <b>          compara dos directorios capturados (0 = iguales, 1 = no)

Los experimentos escriben en rutas relativas (`volcados/...`), así que cada captura
va a su propio directorio y no se pisa con la anterior.

Uso:
    python scripts/diagnostico/contrastar_arnes.py capturar antes
    python scripts/diagnostico/contrastar_arnes.py capturar despues
    python scripts/diagnostico/contrastar_arnes.py comparar antes despues
"""

from __future__ import annotations

import filecmp
import shutil
import subprocess
import sys
from pathlib import Path

RAIZ = Path(__file__).resolve().parents[2]

# Los cinco que cubren las dos rutas de GPU y la PPU por software. Cada uno con sus
# argumentos: `<rom> <secuencia> <cuadro>` (la nativa y la de capas no usan la secuencia).
EXPERIMENTOS = [
    ("ppu_estado", ["{rom}", "{secuencia}", "60"]),
    ("shim_afin_linea", ["{rom}", "{secuencia}", "60"]),
    ("ppu_nativa_afin", ["{rom}"]),
    ("ppu_banda_nativa", ["{rom}", "{secuencia}", "60", "0", "159"]),
    ("ppu_capas_linea", ["{rom}", "{secuencia}", "60"]),
]


def binario(nombre: str) -> Path:
    sufijo = ".exe" if sys.platform == "win32" else ""
    return RAIZ / "target" / "release" / "examples" / f"{nombre}{sufijo}"


def primera_rom() -> Path | None:
    roms = sorted((RAIZ / "roms").glob("*.gba"))
    return roms[0] if roms else None


def capturar(destino: Path, rom: Path, secuencia: Path) -> int:
    nombres = [nombre for nombre, _ in EXPERIMENTOS]
    cmd = ["cargo", "build", "--release", "--features", "proto"]
    for nombre in nombres:
        cmd += ["--example", nombre]
    print("· construyendo:", " ".join(cmd[3:]))
    if subprocess.run(cmd, cwd=RAIZ).returncode != 0:
        print("ERROR: la construcción de los experimentos falló")
        return 1

    if destino.exists():
        shutil.rmtree(destino)
    (destino / "volcados").mkdir(parents=True)

    fallos = 0
    for nombre, argumentos in EXPERIMENTOS:
        exe = binario(nombre)
        if not exe.is_file():
            print(f"ERROR: falta el binario {exe}")
            fallos += 1
            continue
        reales = [a.format(rom=str(rom), secuencia=str(secuencia)) for a in argumentos]
        with (destino / f"{nombre}.out").open("w", encoding="utf-8", newline="\n") as salida:
            proc = subprocess.run(
                [str(exe), *reales], cwd=destino, stdout=salida, stderr=subprocess.STDOUT
            )
        estado = "ok" if proc.returncode == 0 else f"FALLA ({proc.returncode})"
        print(f"· {nombre}: {estado}")
        if proc.returncode != 0:
            fallos += 1

    print(f"\ncaptura en {destino}")
    return 1 if fallos else 0


def comparar(a: Path, b: Path) -> int:
    if not a.is_dir() or not b.is_dir():
        print(f"ERROR: {a} o {b} no son directorios")
        return 1

    ficheros_a = {p.relative_to(a) for p in a.rglob("*") if p.is_file()}
    ficheros_b = {p.relative_to(b) for p in b.rglob("*") if p.is_file()}

    solo_a = sorted(ficheros_a - ficheros_b)
    solo_b = sorted(ficheros_b - ficheros_a)
    comunes = sorted(ficheros_a & ficheros_b)

    distintos = [f for f in comunes if not filecmp.cmp(a / f, b / f, shallow=False)]

    for f in solo_a:
        print(f"SÓLO EN A: {f}")
    for f in solo_b:
        print(f"SÓLO EN B: {f}")
    for f in distintos:
        print(f"DIFERENTE: {f}")

    if solo_a or solo_b or distintos:
        print(
            f"\n{len(distintos)} distintos, {len(solo_a)} sólo en A, {len(solo_b)} sólo en B: "
            "el arnés NO se comporta igual"
        )
        return 1

    print(f"\n{len(comunes)} ficheros idénticos byte a byte: el arnés se comporta igual")
    return 0


def main() -> int:
    if len(sys.argv) < 3:
        print(__doc__)
        return 2
    sub = sys.argv[1]
    if sub == "capturar":
        if len(sys.argv) < 3:
            print("ERROR: falta el directorio de destino")
            return 2
        rom = Path(sys.argv[3]) if len(sys.argv) > 3 else primera_rom()
        if rom is None or not rom.is_file():
            print("ERROR: no encuentro la ROM; pásala como tercer argumento")
            return 1
        secuencia = (
            Path(sys.argv[4])
            if len(sys.argv) > 4
            else RAIZ / "secuencias" / "intro.txt"
        )
        if not secuencia.is_file():
            print(f"ERROR: no encuentro la secuencia {secuencia}")
            return 1
        return capturar(Path(sys.argv[2]), rom, secuencia)
    if sub == "comparar":
        if len(sys.argv) < 4:
            print("ERROR: comparar necesita dos directorios")
            return 2
        return comparar(Path(sys.argv[2]), Path(sys.argv[3]))
    print(f"subcomando desconocido: {sub}")
    return 2


if __name__ == "__main__":
    sys.exit(main())
