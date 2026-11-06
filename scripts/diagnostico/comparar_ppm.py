#!/usr/bin/env python3
r"""
scripts/diagnostico/comparar_ppm.py

Compara dos imágenes en formato PPM P6 (240x160 u otras resoluciones),
calcula métricas de divergencia por píxel y canal, y opcionalmente
genera un mapa de diferencias resaltado.

Códigos de salida:
  0: Imágenes idénticas.
  1: Discrepancias encontradas.
  2: Error de formato, dimensiones incompatibles o archivo no encontrado.
"""

import argparse
import os
import sys
from typing import Optional, Tuple


def leer_ppm(ruta: str) -> Tuple[int, int, int, bytes]:
    """Lee un archivo PPM en formato binario P6.

    Retorna: (ancho, alto, maxval, datos_pixeles_rgb)
    Lanza ValueError si el formato no es válido o está truncado.
    """
    if not os.path.exists(ruta):
        raise FileNotFoundError(f"Archivo no encontrado: {ruta}")

    with open(ruta, "rb") as f:
        datos = f.read()

    longitud = len(datos)
    pos = 0

    def saltar_espacios_y_comentarios(p: int) -> int:
        while p < longitud:
            byte = datos[p : p + 1]
            if byte in b" \t\r\n":
                p += 1
            elif byte == b"#":
                while p < longitud and datos[p : p + 1] != b"\n":
                    p += 1
                if p < longitud and datos[p : p + 1] == b"\n":
                    p += 1
            else:
                break
        return p

    def leer_token(p: int) -> Tuple[bytes, int]:
        p = saltar_espacios_y_comentarios(p)
        inicio = p
        while p < longitud and datos[p : p + 1] not in b" \t\r\n#":
            p += 1
        if inicio == p:
            raise ValueError(f"Encabezado PPM truncado o corrupto en {ruta}")
        token = datos[inicio:p]
        return token, p

    magic, pos = leer_token(pos)
    if magic != b"P6":
        raise ValueError(f"Formato no soportado en {ruta} (esperado P6, obtenido {magic!r})")

    ancho_b, pos = leer_token(pos)
    alto_b, pos = leer_token(pos)
    maxval_b, pos = leer_token(pos)

    try:
        ancho = int(ancho_b)
        alto = int(alto_b)
        maxval = int(maxval_b)
    except ValueError as exc:
        raise ValueError(f"Valores de encabezado inválidos en {ruta}") from exc

    if ancho <= 0 or alto <= 0:
        raise ValueError(f"Dimensiones inválidas en {ruta}: {ancho}x{alto}")
    if maxval <= 0 or maxval > 255:
        raise ValueError(f"Maxval no soportado en {ruta}: {maxval} (debe ser 1..255)")

    # El estándar Netpbm especifica exactamente un carácter whitespace tras maxval
    if pos < longitud and datos[pos : pos + 1] in b" \t\r\n":
        pos += 1
    else:
        raise ValueError(f"Falta delimitador de espacio tras maxval en {ruta}")

    pixeles = datos[pos:]
    esperado = ancho * alto * 3
    if len(pixeles) != esperado:
        raise ValueError(
            f"Tamaño de datos inesperado en {ruta}: se esperaban {esperado} bytes ({ancho}x{alto}x3), "
            f"obtenidos {len(pixeles)}"
        )

    return ancho, alto, maxval, pixeles


leer_ppm_p6 = leer_ppm


def comparar_ppm(
    ruta1: str,
    ruta2: str,
    ruta_diff: Optional[str] = None,
) -> Tuple[int, dict]:
    """Compara dos archivos PPM P6.

    Retorna: (codigo_salida, metricas_dict)
    """
    w1, h1, maxval1, px1 = leer_ppm(ruta1)
    w2, h2, maxval2, px2 = leer_ppm(ruta2)

    if w1 != w2 or h1 != h2:
        raise ValueError(f"Dimensiones incompatibles: {w1}x{h1} vs {w2}x{h2}")

    total_pixeles = w1 * h1
    distintos = 0
    max_dr = 0
    max_dg = 0
    max_db = 0
    suma_deltas = 0.0

    diff_buf = bytearray(total_pixeles * 3) if ruta_diff else None

    for i in range(total_pixeles):
        idx = i * 3
        r1, g1, b1 = px1[idx], px1[idx + 1], px1[idx + 2]
        r2, g2, b2 = px2[idx], px2[idx + 1], px2[idx + 2]

        if r1 == r2 and g1 == g2 and b1 == b2:
            continue

        distintos += 1
        dr = abs(r1 - r2)
        dg = abs(g1 - g2)
        db = abs(b1 - b2)

        if dr > max_dr:
            max_dr = dr
        if dg > max_dg:
            max_dg = dg
        if db > max_db:
            max_db = db

        suma_deltas += (dr + dg + db) / 3.0

        if diff_buf is not None:
            # Resaltado magenta brillante para píxeles divergentes
            diff_buf[idx] = 255
            diff_buf[idx + 1] = 0
            diff_buf[idx + 2] = 255

    identicos = total_pixeles - distintos
    pct_identicos = (identicos / total_pixeles) * 100.0 if total_pixeles > 0 else 0.0
    pct_distintos = (distintos / total_pixeles) * 100.0 if total_pixeles > 0 else 0.0
    delta_medio = (suma_deltas / distintos) if distintos > 0 else 0.0

    if diff_buf is not None and ruta_diff is not None:
        encabezado = f"P6\n{w1} {h1}\n255\n".encode("ascii")
        with open(ruta_diff, "wb") as f_diff:
            f_diff.write(encabezado + diff_buf)

    metricas = {
        "ancho": w1,
        "alto": h1,
        "total_pixeles": total_pixeles,
        "identicos": identicos,
        "pct_identicos": pct_identicos,
        "distintos": distintos,
        "pixeles_diferentes": distintos,
        "pct_distintos": pct_distintos,
        "max_dr": max_dr,
        "max_dg": max_dg,
        "max_db": max_db,
        "delta_medio": delta_medio,
        "ruta_diff": ruta_diff,
    }

    codigo = 0 if distintos == 0 else 1
    return codigo, metricas


def generar_mapa_diferencias(ruta1: str, ruta2: str, ruta_diff: str) -> None:
    """Genera un archivo PPM P6 resaltando las diferencias entre dos imágenes."""
    comparar_ppm(ruta1, ruta2, ruta_diff=ruta_diff)


def imprimir_reporte(metricas: dict) -> None:
    """Imprime el resumen de comparación en consola."""
    w = metricas["ancho"]
    h = metricas["alto"]
    total = metricas["total_pixeles"]
    identicos = metricas["identicos"]
    pct_id = metricas["pct_identicos"]
    distintos = metricas["distintos"]
    pct_dist = metricas["pct_distintos"]
    max_dr = metricas["max_dr"]
    max_dg = metricas["max_dg"]
    max_db = metricas["max_db"]
    delta_medio = metricas["delta_medio"]
    ruta_diff = metricas.get("ruta_diff")

    print(f"Píxeles totales: {total} ({w}x{h})")
    print(f"Píxeles idénticos: {identicos} ({pct_id:.2f}%)")
    print(f"Píxeles diferentes: {distintos} ({pct_dist:.2f}%)")
    print(f"Delta máximo: R={max_dr}, G={max_dg}, B={max_db}")
    print(f"Delta promedio en píxeles divergentes: {delta_medio:.2f}")
    if ruta_diff:
        print(f"Mapa de diferencias guardado en: {ruta_diff}")


def main(argv=None) -> int:
    parser = argparse.ArgumentParser(
        description="Compara dos imágenes PPM P6 y reporta divergencias píxel a píxel."
    )
    parser.add_argument("imagen_a", help="Ruta al primer archivo PPM P6")
    parser.add_argument("imagen_b", help="Ruta al segundo archivo PPM P6")
    parser.add_argument(
        "--diff",
        dest="ruta_diff",
        default=None,
        help="Ruta donde guardar la imagen PPM del mapa de diferencias",
    )

    args = parser.parse_args(argv)

    try:
        codigo, metricas = comparar_ppm(args.imagen_a, args.imagen_b, args.ruta_diff)
        imprimir_reporte(metricas)
        return codigo
    except (FileNotFoundError, ValueError) as exc:
        print(f"ERROR: {exc}", file=sys.stderr)
        return 2
    except Exception as exc:
        print(f"ERROR INESPERADO: {exc}", file=sys.stderr)
        return 2


if __name__ == "__main__":
    sys.exit(main())
