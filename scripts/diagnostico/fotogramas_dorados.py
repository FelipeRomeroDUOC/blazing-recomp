#!/usr/bin/env python3
"""Arnés de fotogramas dorados por escena para regresiones de píxel de la PPU.

Propuesta: `docs/propuestas/2026-10-13_arnes-de-fotogramas-dorados.md`.

Compara el framebuffer que dibuja la PPU contra un dorado capturado antes del cambio
y dice en qué fotograma, en qué píxel y en qué caja difiere, en lugar de limitarse a
comparar un hash global de sesión. Tres subcomandos:

    diferenciar <a.ppm> <b.ppm> [--mapa m.png]   dos fotogramas, píxel a píxel
    capturar    --bin <motor> --rom <rom> --bios <bios>   guarda los dorados
    comparar    --bin <motor> --rom <rom> --bios <bios>   comprueba un cambio

Contrato de códigos de salida, idéntico en los tres:

    0  sin diferencias (o captura correcta)
    1  hay diferencias de píxel: es una regresión
    2  entrada inválida o dorados que no corresponden: no hay veredicto

El motor se usa como caja negra: se le pide un fotograma con `recomp runc --out` y se
lee el PPM P6 que escribe. Ese formato no pierde fidelidad GBA555, porque la expansión
`v8 = v5<<3 | v5>>2` se invierte con `v5 = v8>>3`, así que desde el archivo se
recalcula el framebuffer y el hash FNV-1a que imprime el motor.

Los dorados son píxeles derivados de la ROM, material propietario que AGENTS.md §3
prohíbe versionar: viven en `out/doradas/` (cubierto por `/out/` en `.gitignore`) y lo
que se versiona es el manifiesto de escenas (`escenas_doradas.toml`), que solo lleva
rutas, fotogramas y hashes.

Uso:
    python scripts/diagnostico/fotogramas_dorados.py diferenciar a.ppm b.ppm --mapa m.png
    python scripts/diagnostico/fotogramas_dorados.py capturar --bin <motor> --rom <rom> --bios <bios>
    python scripts/diagnostico/fotogramas_dorados.py comparar --bin <motor> --rom <rom> --bios <bios>
"""

import argparse
import hashlib
import json
import os
import re
import struct
import subprocess
import sys
import tomllib
import zlib

ANCHO = 240
ALTO = 160
PIXELES = ANCHO * ALTO
SALIDA_SIN_DIFERENCIAS = 0
SALIDA_DIFERENCIAS = 1
SALIDA_ERROR = 2

REPO = os.path.dirname(os.path.dirname(os.path.dirname(os.path.abspath(__file__))))
DORADOS_POR_DEFECTO = os.path.join(REPO, "out", "doradas")
ESCENAS_POR_DEFECTO = os.path.join(REPO, "scripts", "diagnostico", "escenas_doradas.toml")
EXTENSION_DLL = "dll" if os.name == "nt" else "so"


class EntradaInvalida(Exception):
    """Entrada que impide emitir un veredicto: siempre termina en código de salida 2."""


def error(mensaje):
    print(f"error: {mensaje}")
    return SALIDA_ERROR


# ------------------------------------------------------------------ PPM y píxeles
def leer_ppm(ruta):
    """Lee un PPM P6 de 240x160 y devuelve los 115200 bytes RGB.

    Cualquier cosa que no sea exactamente el formato que escribe el motor se rechaza:
    un archivo dudoso tiene que producir un error de entrada, nunca un veredicto.
    """
    try:
        with open(ruta, "rb") as f:
            crudo = f.read()
    except OSError as e:
        raise EntradaInvalida(f"{ruta}: no se pudo leer ({e.strerror})") from e

    partes = crudo.split(b"\n", 3)
    if len(partes) != 4 or partes[0].strip() != b"P6":
        hallado = partes[0].decode("ascii", "replace")[:4] if partes else "vacío"
        raise EntradaInvalida(f"{ruta}: se esperaba cabecera P6, se encontró {hallado!r}")

    cuerpo = partes[3]
    if len(cuerpo) != PIXELES * 3:
        raise EntradaInvalida(
            f"{ruta}: {len(cuerpo) // 3} píxeles, se esperaban {PIXELES} "
            f"({ANCHO}x{ALTO}, {PIXELES * 3} bytes)"
        )
    return cuerpo


def fnv_gba555(rgb):
    """Recalcula el hash FNV-1a de framebuffer que imprime el motor, desde el PPM.

    `dump_frame` expande cada canal de 5 bits como `v8 = v5<<3 | v5>>2` y escribe RGB;
    `v5 = v8>>3` devuelve el canal exacto, así que se reconstruye el u16 GBA555 y se
    aplica la misma función de hash que `fb_hash` en el motor.
    """
    h = 0xCBF2_9CE4_8422_2325
    for i in range(0, len(rgb), 3):
        px = (rgb[i] >> 3) | ((rgb[i + 1] >> 3) << 5) | ((rgb[i + 2] >> 3) << 10)
        for byte in px.to_bytes(2, "little"):
            h ^= byte
            h = (h * 0x0100_0001B3) & 0xFFFFFFFFFFFFFFFF
    return f"{h:016x}"


def png_rgb(rgb, ancho, alto):
    """Codifica un PNG RGB8 con la biblioteca estándar, sin dependencias externas."""
    filas = bytearray()
    for y in range(alto):
        filas.append(0)  # filtro None: el mapa es casi plano y no compensa comprimir por fila
        filas.extend(rgb[y * ancho * 3 : (y + 1) * ancho * 3])

    def bloque(tipo, datos):
        return (
            struct.pack(">I", len(datos))
            + tipo
            + datos
            + struct.pack(">I", zlib.crc32(tipo + datos) & 0xFFFFFFFF)
        )

    return (
        b"\x89PNG\r\n\x1a\n"
        + bloque(b"IHDR", struct.pack(">IIBBBBB", ancho, alto, 8, 2, 0, 0, 0))
        + bloque(b"IDAT", zlib.compress(bytes(filas), 9))
        + bloque(b"IEND", b"")
    )


def diferenciar_ppm(a, b):
    """Compara dos fotogramas; devuelve (informe, lista de índices distintos) o (None, [])."""
    distintos = [i for i in range(PIXELES) if a[i * 3 : i * 3 + 3] != b[i * 3 : i * 3 + 3]]
    if not distintos:
        return None, []

    columnas = [i % ANCHO for i in distintos]
    filas = [i // ANCHO for i in distintos]
    x0, x1 = min(columnas), max(columnas)
    y0, y1 = min(filas), max(filas)
    primer = distintos[0]
    tiles_afectados = len({((i % ANCHO) // 8, (i // ANCHO) // 8) for i in distintos})
    informe = (
        f"DIFIERE {len(distintos)} px | "
        f"primero columna={primer % ANCHO} fila={primer // ANCHO} | "
        f"caja x{x0}-{x1} y{y0}-{y1} | "
        f"filas {y0}-{y1} | tiles 8x8 afectados {tiles_afectados}/600"
    )
    return informe, distintos


def mapa_de_diferencias(a, distintos):
    """Gris medio para el píxel igual, rojo puro para el que difiere."""
    grises = bytearray(PIXELES * 3)
    for i in range(PIXELES):
        v = (a[i * 3] + a[i * 3 + 1] + a[i * 3 + 2]) // 6
        grises[i * 3] = grises[i * 3 + 1] = grises[i * 3 + 2] = v
    for i in distintos:
        grises[i * 3 : i * 3 + 3] = b"\xff\x00\x00"
    return bytes(grises)


# ----------------------------------------------------------------------- motor
def sha256_archivo(ruta):
    h = hashlib.sha256()
    with open(ruta, "rb") as f:
        for trozo in iter(lambda: f.read(1 << 20), b""):
            h.update(trozo)
    return h.hexdigest()


def dll_de(rom):
    """Biblioteca recompilada que `recomp runc` carga para esa ROM."""
    stem = os.path.splitext(os.path.basename(rom))[0]
    return os.path.join(REPO, "out", f"{stem}-bios.{EXTENSION_DLL}")


def ejecutar_ancla(binario, rom, bios, script, ancla, destino):
    """Corre `recomp runc` hasta el fotograma pedido y devuelve (hash, bloques, fallback)."""
    dll = dll_de(rom)
    if not os.path.isfile(dll):
        raise EntradaInvalida(
            f"falta la biblioteca recompilada {os.path.relpath(dll, REPO)}; "
            f"ejecuta `recomp build` para esa ROM antes de usar el arnés"
        )
    hecho = subprocess.run(
        [
            binario,
            "runc",
            rom,
            "--bios",
            bios,
            "--frames",
            str(ancla),
            "--input",
            script,
            "--out",
            destino,
        ],
        cwd=REPO,
        capture_output=True,
        text=True,
    )
    if hecho.returncode != 0:
        ultima = (hecho.stderr.strip().splitlines() or ["sin salida"])[-1]
        raise EntradaInvalida(
            f"el motor terminó con {hecho.returncode} en el fotograma {ancla}: {ultima}"
        )
    texto = hecho.stdout + hecho.stderr
    hash_motor = re.search(r"hash: ([0-9a-f]+)", texto)
    bloques = re.search(r"native_blocks: (\d+)", texto)
    fallback = re.search(r"fallback_steps: (\d+)", texto)
    if not (hash_motor and bloques and fallback):
        raise EntradaInvalida(
            f"la salida del motor no trae hash/bloques/fallback en el fotograma {ancla}"
        )
    return hash_motor.group(1), int(bloques.group(1)), int(fallback.group(1))


def leer_escenas(ruta):
    """Lee el manifiesto de escenas versionado: nombre, guion y anclas por escena."""
    try:
        with open(ruta, "rb") as f:
            datos = tomllib.load(f)
    except OSError as e:
        raise EntradaInvalida(f"{ruta}: no se pudo leer ({e.strerror})") from e
    except tomllib.TOMLDecodeError as e:
        raise EntradaInvalida(f"{ruta}: TOML inválido ({e})") from e

    base = os.path.dirname(os.path.abspath(ruta))
    escenas = []
    for entrada in datos.get("escena", []):
        nombre = entrada.get("nombre")
        script = entrada.get("script")
        anclas = entrada.get("anclas")
        if not nombre or not script or not anclas:
            raise EntradaInvalida(
                f"{ruta}: cada escena necesita `nombre`, `script` y `anclas`"
            )
        escena = {
            "nombre": nombre,
            "script": os.path.join(base, script),
            "anclas": [int(a) for a in anclas],
        }
        if not os.path.isfile(escena["script"]):
            raise EntradaInvalida(f"{escena['script']}: el guion de la escena no existe")
        escenas.append(escena)
    if not escenas:
        raise EntradaInvalida(f"{ruta}: no hay ninguna tabla `[[escena]]`")
    return escenas


def ruta_manifesto(dorados):
    return os.path.join(dorados, "manifesto.json")


# ------------------------------------------------------------------ subcomandos
def cmd_diferenciar(args):
    try:
        a = leer_ppm(args.a)
        b = leer_ppm(args.b)
    except EntradaInvalida as e:
        return error(e)

    informe, distintos = diferenciar_ppm(a, b)
    if informe is None:
        print(f"sin diferencias ({PIXELES} píxeles, {ANCHO}x{ALTO})")
        return SALIDA_SIN_DIFERENCIAS

    if args.mapa:
        rgb = mapa_de_diferencias(a, distintos)
        try:
            with open(args.mapa, "wb") as f:
                f.write(png_rgb(rgb, ANCHO, ALTO))
        except OSError as e:
            return error(f"{args.mapa}: no se pudo escribir ({e.strerror})")
        print(f"{informe} | mapa: {args.mapa}")
    else:
        print(informe)
    return SALIDA_DIFERENCIAS


def cmd_capturar(args):
    try:
        escenas = leer_escenas(args.escenas)
        dll = dll_de(args.rom)
        os.makedirs(args.dorados, exist_ok=True)
        manifesto = {
            "version": 1,
            "rom_sha256": sha256_archivo(args.rom),
            "bios_sha256": sha256_archivo(args.bios),
            "script_sha256": {e["nombre"]: sha256_archivo(e["script"]) for e in escenas},
            "biblioteca_sha256": sha256_archivo(dll) if os.path.isfile(dll) else None,
            "anclas": {},
        }
    except (EntradaInvalida, OSError) as e:
        return error(e)

    total = 0
    for escena in escenas:
        print(f"escena {escena['nombre']} (guion {os.path.basename(escena['script'])})")
        for ancla in escena["anclas"]:
            destino = os.path.join(args.dorados, f"{escena['nombre']}_{ancla}.ppm")
            try:
                hash_motor, bloques, fallback = ejecutar_ancla(
                    args.bin, args.rom, args.bios, escena["script"], ancla, destino
                )
                rgb = leer_ppm(destino)
            except EntradaInvalida as e:
                return error(e)
            recalculado = fnv_gba555(rgb)
            if recalculado != hash_motor:
                return error(
                    f"{os.path.basename(destino)}: el PPM recalcula {recalculado} "
                    f"pero el motor imprimió {hash_motor}; el archivo no es el fotograma pedido"
                )
            manifesto["anclas"][f"{escena['nombre']}@{ancla}"] = {
                "ppm": os.path.basename(destino),
                "hash": hash_motor,
                "bloques": bloques,
                "fallback": fallback,
            }
            total += 1
            print(f"  ancla {ancla}: {hash_motor} ({bloques} bloques, {fallback} fallback)")

    try:
        with open(ruta_manifesto(args.dorados), "w", encoding="utf-8") as f:
            json.dump(manifesto, f, indent=2, sort_keys=True)
            f.write("\n")
    except OSError as e:
        return error(f"{ruta_manifesto(args.dorados)}: no se pudo escribir ({e.strerror})")
    print(f"capturados {total} dorados en {args.dorados}")
    return SALIDA_SIN_DIFERENCIAS


def cmd_comparar(args):
    try:
        escenas = leer_escenas(args.escenas)
        ruta = ruta_manifesto(args.dorados)
        if not os.path.isfile(ruta):
            raise EntradaInvalida(f"{ruta}: no hay dorados capturados; ejecuta `capturar` primero")
        with open(ruta, encoding="utf-8") as f:
            manifesto = json.load(f)
        actuales = {
            "rom": sha256_archivo(args.rom),
            "bios": sha256_archivo(args.bios),
            "script": {e["nombre"]: sha256_archivo(e["script"]) for e in escenas},
        }
    except (EntradaInvalida, OSError, json.JSONDecodeError) as e:
        return error(e)

    # Los dorados solo valen para la misma ROM, BIOS y guion: si algo cambió, la
    # comparación produciría diferencias falsas y hay que rechazarse nombrando el campo.
    campos = []
    for campo, actual in (("rom", actuales["rom"]), ("bios", actuales["bios"])):
        guardado = manifesto.get(f"{campo}_sha256")
        if guardado and guardado != actual:
            campos.append(campo)

    # El guion se guarda por escena; un manifiesto que no trae ninguno no puede juzgarse.
    guiones_guardados = manifesto.get("script_sha256")
    if isinstance(guiones_guardados, dict) and guiones_guardados:
        if any(guiones_guardados.get(n) != v for n, v in actuales["script"].items()):
            campos.append("script")
    elif isinstance(guiones_guardados, str) and guiones_guardados:
        # Manifiesto antiguo con un solo guion: tiene que coincidir con todos los actuales.
        if any(guiones_guardados != v for v in actuales["script"].values()):
            campos.append("script")

    if campos:
        return error(
            "los dorados no corresponden: cambió "
            + ", ".join(campos)
            + " respecto al momento de `capturar`; vuelve a capturarlos"
        )

    dll = dll_de(args.rom)
    dll_actual = sha256_archivo(dll) if os.path.isfile(dll) else None
    if manifesto.get("biblioteca_sha256") and dll_actual and manifesto["biblioteca_sha256"] != dll_actual:
        print(
            "aviso: la biblioteca recompilada cambió respecto a los dorados; "
            "las diferencias pueden venir de ahí y no de la PPU"
        )

    fallos = []
    for escena in escenas:
        print(f"escena {escena['nombre']}")
        for ancla in escena["anclas"]:
            clave = f"{escena['nombre']}@{ancla}"
            entrada = manifesto.get("anclas", {}).get(clave)
            if entrada is None:
                return error(f"{clave}: falta el dorado; ejecuta `capturar`")
            dorado = os.path.join(args.dorados, entrada["ppm"])
            if not os.path.isfile(dorado):
                return error(f"{dorado}: falta el dorado; ejecuta `capturar`")
            destino = os.path.join(args.dorados, f"_{escena['nombre']}_{ancla}.actual.ppm")
            try:
                hash_motor, _, _ = ejecutar_ancla(
                    args.bin, args.rom, args.bios, escena["script"], ancla, destino
                )
                rgb_dorado = leer_ppm(dorado)
                rgb_actual = leer_ppm(destino)
            except EntradaInvalida as e:
                return error(e)

            informe, distintos = diferenciar_ppm(rgb_dorado, rgb_actual)
            if informe is None:
                print(f"  ancla {ancla}: OK ({hash_motor})")
                continue

            mapa = os.path.join(args.dorados, f"{escena['nombre']}_{ancla}.diff.png")
            try:
                with open(mapa, "wb") as f:
                    f.write(
                        png_rgb(mapa_de_diferencias(rgb_dorado, distintos), ANCHO, ALTO)
                    )
            except OSError as e:
                return error(f"{mapa}: no se pudo escribir ({e.strerror})")
            print(f"  ancla {ancla}: {informe} | mapa: {mapa}")
            fallos.append((clave, len(distintos)))

    if fallos:
        resumen = ", ".join(f"{clave} ({n} px)" for clave, n in fallos)
        print(f"\n{len(fallos)} ancla(s) difieren, {sum(n for _, n in fallos)} píxeles: {resumen}")
        return SALIDA_DIFERENCIAS
    print("\ntodos los anclas coinciden")
    return SALIDA_SIN_DIFERENCIAS


def main():
    parser = argparse.ArgumentParser(
        description="Arnés de fotogramas dorados por escena (regresiones de píxel de la PPU).",
        epilog="Códigos de salida: 0 sin diferencias, 1 diferencias de píxel, 2 entrada inválida.",
    )
    sub = parser.add_subparsers(dest="subcomando", required=True)

    d = sub.add_parser("diferenciar", help="compara dos PPM P6 240x160")
    d.add_argument("a", help="fotograma de referencia")
    d.add_argument("b", help="fotograma a comparar")
    d.add_argument("--mapa", help="ruta del PNG con las diferencias en rojo")
    d.set_defaults(func=cmd_diferenciar)

    def comunes(p, ayuda_dorados):
        p.add_argument("--bin", required=True, help="binario del motor (recomp)")
        p.add_argument("--rom", required=True, help="ROM oficial")
        p.add_argument("--bios", required=True, help="BIOS oficial")
        p.add_argument("--escenas", default=ESCENAS_POR_DEFECTO, help="manifiesto de escenas TOML")
        p.add_argument("--dorados", default=DORADOS_POR_DEFECTO, help=ayuda_dorados)

    c = sub.add_parser("capturar", help="guarda los dorados del manifiesto de escenas")
    comunes(c, "carpeta local donde escribir los dorados")
    c.set_defaults(func=cmd_capturar)

    p = sub.add_parser("comparar", help="comprueba un binario contra los dorados")
    comunes(p, "carpeta local donde están los dorados")
    p.set_defaults(func=cmd_comparar)

    args = parser.parse_args()
    return args.func(args)


if __name__ == "__main__":
    sys.exit(main())
