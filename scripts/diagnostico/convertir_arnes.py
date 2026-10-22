#!/usr/bin/env python3
"""Convierte un experimento de `examples/` al arnés común `examples/comun/`.

Por fichero:
  1. quita las copias locales (funciones, struct/impl Gpu, constantes, `use` del pipeline),
  2. renombra las llamadas cuya forma del módulo se llama distinto,
  3. añade un `use` por identificador que el fichero siga usando,
  4. inserta `#[path = "comun/mod.rs"] mod comun;`.

Se usan `use` y no rutas completas a propósito: `{ALTO}` dentro de un `format!` es una
captura del nombre, y `{comun::gpu::ALTO}` no compila.

Es la herramienta que hizo la migración de los 47 (propuesta
`docs/propuestas/2026-10-22_arnes-de-experimentos-comun.md`). Sirve para lo mismo que
servía entonces: cuando entra un experimento nuevo con el arnés copiado, esto se lo
convierte. Un fichero que ya declara `mod comun;` se salta, así que el script es
idempotente y no se puede romper por volver a lanzarlo.

Uso:
    python scripts/diagnostico/convertir_arnes.py --solo shim_nuevo,ppu_nueva
    python scripts/diagnostico/convertir_arnes.py
"""

import argparse
import pathlib
import re
import sys

RAIZ = pathlib.Path(__file__).resolve().parents[2]
EXAMPLES = RAIZ / "examples"

# `ppu_capas_linea` conserva su `dispcnt_de` de 11 variantes: no es una copia, resuelve
# las variantes desde una tabla. Los 47 declaran el módulo; nadie queda fuera.
CONSERVAR_FUNCS = {"ppu_capas_linea.rs": {"dispcnt_de"}}

FUNCIONES = [
    "leer_secuencia",
    "canal_8bit",
    "cerca",
    "io16",
    "dispcnt_de",
    "io_con_variante",
    "software_a_rgb",
]
CONSTANTES = ["BOTONES", "ANCHO", "ALTO", "FILA_GPU", "TOLERANCIA", "BIT_OBJ", "BITS_CAPAS"]

# identificador final -> (módulo, nombre exportado)
EXPORTA = {
    "leer_secuencia": ("entrada", "leer_secuencia"),
    "leer_tramos": ("entrada", "leer_tramos"),
    "BOTONES": ("entrada", "BOTONES"),
    "canal_8bit": ("color", "canal_8bit"),
    "cerca": ("color", "cerca"),
    "cerca_pixel": ("color", "cerca_pixel"),
    "software_a_rgb": ("color", "software_a_rgb"),
    "TOLERANCIA": ("color", "TOLERANCIA"),
    "io16": ("registro", "io16"),
    "dispcnt_de": ("registro", "dispcnt_de"),
    "io_con_variante": ("registro", "io_con_variante"),
    "BIT_OBJ": ("registro", "BIT_OBJ"),
    "BITS_CAPAS": ("registro", "BITS_CAPAS"),
    "ANCHO": ("gpu", "ANCHO"),
    "ALTO": ("gpu", "ALTO"),
    "FILA_GPU": ("gpu", "FILA_GPU"),
    "Gpu": ("gpu", "Gpu"),
}

# `ppu_frente_a_frente` declara `ANCHO`/`ALTO` como `u32`, no como `usize`: se quedan
# de su lado para no cambiar los tipos de sus bucles.
CONSERVAR_CONSTS = {"ppu_frente_a_frente.rs": {"ANCHO", "ALTO"}}

# Líneas sueltas que se van: `ppu_frente_a_frente` tenía `io16` copiado como closure
# dentro de `main`, con la misma firma que la del módulo.
LINEAS_FUERA = {
    "ppu_frente_a_frente.rs": [r"let io16 = \|"],
}

# Reemplazos que sólo tienen sentido en un fichero concreto.
PARA_ESPECIAL = {
    "ppu_compare.rs": [
        (r"renderiza\(&m\)", "renderiza(&m.bus.vram, &m.bus.oam, &m.bus.palette, &m.bus.io)"),
    ],
}

# llamada local -> llamada del módulo, cuando el nombre cambia
RENOMBRA_LLAMADA = {
    "cerca_pixel": (r"\bcerca\(", "cerca_pixel("),
    "leer_tramos": (r"\bleer_secuencia\(", "leer_tramos("),
    "Gpu::nueva_nativa": (r"\bGpu::nueva\(", "Gpu::nueva_nativa("),
}


def sin_cadenas(linea):
    """Quita los literales de cadena para contar llaves sin equivocarse."""
    return re.sub(r'"(?:[^"\\]|\\.)*"', '""', linea)


def fuera_de_cadenas(linea, patron, reemplazo):
    """Aplica un reemplazo solo al código, nunca dentro de un literal."""
    partes = re.split(r'("(?:[^"\\]|\\.)*")', linea)
    for k in range(0, len(partes), 2):
        partes[k] = re.sub(patron, reemplazo, partes[k])
    return "".join(partes)


def sin_comentarios(linea):
    """Quita los comentarios, sin tocar el texto de dentro de un literal."""
    partes = re.split(r'("(?:[^"\\]|\\.)*")', linea)
    for k in range(0, len(partes), 2):
        partes[k] = partes[k].split("//", 1)[0]
    return "".join(partes)


def quitar_item(lineas, cabecera, es_const=False):
    """Quita el ítem cuya primera línea casa con `cabecera` (anclada al inicio).

    No cuenta llaves: en este código rustfmt cierra cada ítem de primer nivel con una
    `}` sola en columna 0, y un `const` acaba en la primera línea que termina en `;`.
    Contar llaves borró de más (`fn rgb8` de `ppu_nativa_compara`).
    """
    patron = re.compile(cabecera)
    for i, linea in enumerate(lineas):
        if not patron.match(linea):
            continue
        fin = None
        for j in range(i, len(lineas)):
            if es_const:
                if lineas[j].rstrip().endswith(";"):
                    fin = j + 1
                    break
            elif lineas[j] == "}":
                fin = j + 1
                break
        if fin is None:
            raise SystemExit(f"no se encontró el final de {cabecera} en la línea {i + 1}")
        inicio = i
        while inicio > 0 and lineas[inicio - 1].lstrip().startswith("///"):
            inicio -= 1
        while inicio > 0 and not lineas[inicio - 1].strip():
            inicio -= 1
        del lineas[inicio:fin]
        return True
    return False


def apretar_blancos(lineas, desde, hasta):
    """Deja un solo blanco seguido dentro de la ventana [desde, hasta).

    Se aplica sólo alrededor del bloque insertado: el resto del fichero conserva
    sus blancos tal como estaban (hay ficheros con dos seguidos a propósito).
    """
    i = max(1, desde)
    fin = min(hasta, len(lineas))
    while i < fin:
        if not lineas[i].strip() and not lineas[i - 1].strip():
            del lineas[i]
            fin -= 1
        else:
            i += 1


def poda_use_del_motor(lineas):
    """Quita de los `use` del motor los nombres que el fichero ya no usa.

    Hay experimentos que construyen su propio pipeline (`ppu_proto`) y otros que sólo
    citan el pipeline en su `//!` de cabecera; por eso se mira el código, no el nombre.
    """
    cuerpo = "\n".join(sin_comentarios(l) for l in lineas if not l.startswith("use "))
    fuera = []
    for linea in lineas:
        m = re.match(r"^use blazing_recomp::ppu_(?:shim|nativa)::(\{[^}]*\}|\w+);$", linea)
        if not m:
            fuera.append(linea)
            continue
        resto = m.group(1)
        if not resto.startswith("{"):
            if re.search(rf"\b{resto}\b", cuerpo):
                fuera.append(linea)
            continue
        items = [x.strip() for x in resto[1:-1].split(",") if x.strip()]
        quedan = [x for x in items if re.search(rf"\b{x}\b", cuerpo)]
        if quedan:
            fuera.append(linea.replace(resto, "{" + ", ".join(quedan) + "}"))
    return fuera


def convertir(ruta):
    texto = ruta.read_text(encoding="utf-8")
    lineas = texto.splitlines()

    es_nativa = "PpuNativaPipeline" in texto
    firma_cerca = re.search(r"fn cerca\(([^)]*)\)", texto)
    es_pixel = firma_cerca is not None and "usize" not in firma_cerca.group(1)
    usa_tramos = ruta.name == "ppu_compare.rs"

    quitados = []
    se_quedan_funcs = CONSERVAR_FUNCS.get(ruta.name, set())
    for nombre in FUNCIONES:
        if nombre in se_quedan_funcs:
            continue
        if quitar_item(lineas, rf"fn {nombre}\("):
            quitados.append(f"fn {nombre}")
    se_quedan = CONSERVAR_CONSTS.get(ruta.name, set())
    for nombre in CONSTANTES:
        if nombre in se_quedan:
            continue
        if quitar_item(lineas, rf"const {nombre}\b", es_const=True):
            quitados.append(f"const {nombre}")
    for cabecera in (r"struct Gpu ", r"impl Gpu \{"):
        if quitar_item(lineas, cabecera):
            quitados.append(cabecera.strip())
    # `ppu_compare` declaraba su propio `Tramo`; ahora lo trae `leer_tramos`.
    if usa_tramos and quitar_item(lineas, r"struct Tramo "):
        quitados.append("struct Tramo")

    # Renombra sólo el código: `{ALTO}` en un `format!` no se toca.
    pendientes = []
    if es_pixel:
        pendientes.append(RENOMBRA_LLAMADA["cerca_pixel"])
    if usa_tramos:
        pendientes.append(RENOMBRA_LLAMADA["leer_tramos"])
    if es_nativa:
        pendientes.append(RENOMBRA_LLAMADA["Gpu::nueva_nativa"])
    for patron, reemplazo in pendientes:
        lineas = [fuera_de_cadenas(l, patron, reemplazo) for l in lineas]
    for patron, reemplazo in PARA_ESPECIAL.get(ruta.name, []):
        lineas = [fuera_de_cadenas(l, patron, reemplazo) for l in lineas]
    for patron in LINEAS_FUERA.get(ruta.name, []):
        fuera = [l for l in lineas if re.search(patron, l)]
        if fuera:
            quitados.append(f"línea local ({len(fuera)})")
            lineas = [l for l in lineas if not re.search(patron, l)]
    lineas = poda_use_del_motor(lineas)

    # Qué identificadores del módulo usa todavía este fichero (las capturas cuentan).
    cuerpo = "\n".join(sin_comentarios(l) for l in lineas)
    por_modulo = {}
    for ident, (modulo, nombre) in EXPORTA.items():
        if ident in se_quedan or ident in se_quedan_funcs:
            continue
        # Los comentarios no cuentan como uso: un `io16` citado en prosa no necesita import.
        if re.search(rf"\b{re.escape(ident)}\b", cuerpo):
            por_modulo.setdefault(modulo, set()).add(nombre)

    imports = []
    for modulo in ("color", "entrada", "gpu", "registro"):
        nombres = por_modulo.get(modulo)
        if not nombres:
            continue
        if len(nombres) == 1:
            imports.append(f"use comun::{modulo}::{nombres.pop()};")
        else:
            imports.append(f"use comun::{modulo}::{{{', '.join(sorted(nombres))}}};")

    primer_codigo = next(
        (i for i, l in enumerate(lineas) if l.strip() and not l.lstrip().startswith("//")),
        len(lineas),
    )
    usos = [i for i, l in enumerate(lineas) if l.startswith("use ")]
    corte = usos[-1] if usos else primer_codigo - 1
    bloque = ["", '#[path = "comun/mod.rs"]', "mod comun;", ""] + imports + [""]
    for desplazamiento, linea in enumerate(bloque):
        lineas.insert(corte + 1 + desplazamiento, linea)
    apretar_blancos(lineas, corte - 1, corte + len(bloque) + 3)

    # `newline="\n"`: sin esto Python traduce a CRLF en Windows y el árbol de trabajo se
    # llena de finales de línea distintos a los del resto del repositorio.
    with ruta.open("w", encoding="utf-8", newline="\n") as f:
        f.write("\n".join(lineas) + "\n")
    return quitados, imports


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--solo", default="", help="lista de stems separados por coma")
    args = parser.parse_args()

    if args.solo:
        objetivos = [EXAMPLES / f"{s.strip()}.rs" for s in args.solo.split(",") if s.strip()]
    else:
        objetivos = sorted(EXAMPLES.glob("*.rs"))

    convertidos = 0
    for ruta in objetivos:
        if not ruta.is_file():
            print(f"FALTA {ruta}")
            sys.exit(1)
        if "mod comun;" in ruta.read_text(encoding="utf-8"):
            # Ya está convertido: volver a convertirlo duplicaría los `use`.
            print(f"{ruta.name}: ya usa el módulo común, se salta")
            continue
        quitados, imports = convertir(ruta)
        convertidos += 1
        print(f"{ruta.name}: {len(quitados)} fuera | {len(imports)} use")
    print(f"\n{convertidos} fichero(s) convertidos")


if __name__ == "__main__":
    main()
