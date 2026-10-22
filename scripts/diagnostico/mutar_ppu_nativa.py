#!/usr/bin/env python3
"""Mutación controlada de la PPU nativa para comprobar que los tests la detectan.

El script:
  1. lee el contenido original de cada fichero mutado en memoria,
  2. aplica una mutación (un cambio concreto),
  3. ejecuta el test de aceptación,
  4. restaura el contenido original en un bloque `finally`,
  5. comprueba que el test falló con la mutación (es decir, que el test
     detecta el fallo) y que vuelve a pasar sin ella.

No usa comandos destructivos de Git: la restauración es en memoria y sobre el
disco directamente, para no perder trabajo sin commitear.

Uso:
  python scripts/diagnostico/mutar_ppu_nativa.py
"""

from __future__ import annotations

import hashlib
import subprocess
import sys
from pathlib import Path

RAIZ = Path(__file__).resolve().parents[2]

# Arnés sintético del fondo afín: monta un estado de fondo afín y lo compara
# con la referencia por software. Es lo que hay que volver a correr antes de
# comprobar las mutaciones del shader, porque su test lee el informe que
# escribe (`volcados/ppu-nativa-afin.txt`).
ARNES_AFIN = [
    "cargo",
    "run",
    "--release",
    "--features",
    "proto",
    "--example",
    "ppu_nativa_afin",
]

# Informes que escribe un arnés y que hay que dejar como estaban.
ARTEFACTOS = ["volcados/ppu-nativa-afin.txt"]

# (nombre, fichero relativo, texto original, texto mutado, objetivo, test,
#  preparar)
#
# `objetivo` es el target de `cargo test`: `lib` para los tests satélite de
# `src/ppu_nativa_tests.rs` o el nombre del fichero de `tests/` para los de
# integración. `test` es el nombre completo del test, que debe morir con la
# mutación. `preparar` es un comando opcional que hay que correr **antes** del
# test: los tests de píxel leen el informe que escribe el arnés, así que sin
# regenerarlo leería el informe viejo y la mutación pasaría desapercibida.
MUTACIONES = [
    (
        "alfa_sin_redondeo_a_media",
        "src/ppu_nativa.rs",
        "    let r = ((ar * eva + br * evb + 8) >> 4).min(31);",
        "    let r = ((ar * eva + br * evb) >> 4).min(31);",
        "ppu_nativa_tests",
        "test_alfa_coeficientes_intermedios",
        None,
    ),
    (
        "verde_en_carril_de_5_bits",
        "src/ppu_nativa.rs",
        "        (((c >> 5) & 31) as u32) << 1,",
        "        ((c >> 5) & 31) as u32,",
        "ppu_nativa_tests",
        "test_alfa_coeficientes_intermedios",
        None,
    ),
    (
        "aclarar_sin_saturar",
        "src/ppu_nativa.rs",
        "    join6(\n        r + (((31 - r) * evy + 8) >> 4),",
        "    join6(\n        r + (((31 - r) * evy + 8) >> 4).min(30),",
        "ppu_nativa_tests",
        "test_extremos_de_la_mezcla",
        None,
    ),
    # --- La lista de sprites no puede verse desde la etapa de vértices ---
    #
    # Esta es la mutación que reproduce el crash del dispositivo: un
    # *storage buffer* visible en el vértice exige la capacidad
    # `DownlevelFlags::VERTEX_STORAGE`, que el estándar de OpenGL ES permite
    # no tener.
    (
        "sprites_visibles_en_la_etapa_de_vertices",
        "src/ppu_nativa.rs",
        "        entrada_storage(BINDING_SPRITES, wgpu::ShaderStages::FRAGMENT),",
        "        entrada_storage(BINDING_SPRITES, wgpu::ShaderStages::VERTEX_FRAGMENT),",
        "lib",
        "ppu_nativa::tests::ningun_storage_buffer_es_visible_en_la_etapa_de_vertices",
        None,
    ),
    (
        "el_vertices_lee_la_lista_de_sprites",
        "src/ppu_nativa.wgsl",
        "    let px = inst_geometria.x + quad.x * inst_geometria.z;\n"
        "    let py = inst_geometria.y + quad.y * inst_geometria.w;",
        "    let s = sprites[inst];\n"
        "    let px = s.pos_x + quad.x * s.caja_x;\n"
        "    let py = s.pos_y + quad.y * s.caja_y;",
        "ppu_nativa_tests",
        "ningun_punto_de_entrada_de_vertices_lee_la_lista_de_sprites",
        None,
    ),
    (
        "los_atributos_de_instancia_se_mueven_de_ubicacion",
        "src/ppu_nativa.wgsl",
        "    @location(1) inst_geometria: vec4<f32>,\n"
        "    @location(2) inst_clave: vec2<u32>,",
        "    @location(3) inst_geometria: vec4<f32>,\n"
        "    @location(4) inst_clave: vec2<u32>,",
        "ppu_nativa_tests",
        "ningun_punto_de_entrada_de_vertices_lee_la_lista_de_sprites",
        None,
    ),
    (
        "el_buffer_de_instancias_no_lleva_la_clave",
        "src/ppu_nativa.rs",
        "            clave: [sprite.prioridad, sprite.oam_index],",
        "            clave: [0, 0],",
        "lib",
        "ppu_nativa::tests::el_buffer_de_instancias_lleva_lo_que_el_vertices_necesita",
        None,
    ),
    (
        "el_buffer_de_instancias_tiene_otra_zancada",
        "src/ppu_nativa.rs",
        "        array_stride: std::mem::size_of::<SpriteInstanciaGpu>() as wgpu::BufferAddress,",
        "        array_stride: std::mem::size_of::<SpriteInstanciaGpu>() as wgpu::BufferAddress + 4,",
        "lib",
        "ppu_nativa::tests::el_buffer_de_instancias_lleva_lo_que_el_vertices_necesita",
        None,
    ),
    # --- El camino del fondo afín ---
    #
    # Las dos primeras mutaciones son del shader: las detecta el arnés
    # sintético, que hay que volver a correr porque su test lee el informe.
    # La tercera es de la decodificación en Rust: la detecta el test unitario
    # de los signos.
    (
        "afin_desplazamiento_de_8_bits",
        "src/ppu_nativa.wgsl",
        "    var tx = fx >> 8;\n    var ty = fy >> 8;",
        "    var tx = fx >> 7;\n    var ty = fy >> 7;",
        "ppu_nativa_tests",
        "test_fondo_afin_casos_limite_coinciden",
        ARNES_AFIN,
    ),
    (
        "afin_pb_y_pd_intercambiados",
        "src/ppu_nativa.wgsl",
        "    let fx = c.afin_ref_x + linea * c.afin_pb + i * c.afin_pa;\n"
        "    let fy = c.afin_ref_y + linea * c.afin_pd + i * c.afin_pc;",
        "    let fx = c.afin_ref_x + linea * c.afin_pd + i * c.afin_pa;\n"
        "    let fy = c.afin_ref_y + linea * c.afin_pb + i * c.afin_pc;",
        "ppu_nativa_tests",
        "test_fondo_afin_casos_limite_coinciden",
        ARNES_AFIN,
    ),
    (
        "afin_ancla_sin_extension_de_signo",
        "src/ppu_nativa.rs",
        "        let signo28 = |v: u32| -> i32 { ((v << 4) as i32) >> 4 };",
        "        let signo28 = |v: u32| -> i32 { v as i32 };",
        "lib",
        "ppu_nativa::tests::los_registros_del_fondo_afin_llevan_el_signo",
        None,
    ),
]


def escribir(ruta: Path, contenido: str) -> None:
    """Escribe el contenido sin traducir los finales de línea.

    `Path.write_text` en Windows convierte los `\n` en `\r\n`, y el fichero
    acabaría con finales distintos de los que tenía. Con `newline=""` se
    escribe el contenido tal cual.
    """
    with open(ruta, "w", encoding="utf-8", newline="") as f:
        f.write(contenido)


def ejecutar(objetivo: str, test: str) -> tuple[int, str]:
    if objetivo == "lib":
        args = ["cargo", "test", "--release", "--lib", "--", test, "--exact"]
    else:
        args = ["cargo", "test", "--release", "--test", objetivo, "--", test, "--exact"]
    proc = subprocess.run(args, cwd=RAIZ, capture_output=True, text=True)
    return proc.returncode, proc.stdout + proc.stderr


def preparar(informe: list[str] | None) -> tuple[bool, str]:
    """Corre el arnés que regenera el informe que lee el test.

    Devuelve `(hecho, mensaje)`: si el arnés no pudo correr (por ejemplo, sin
    GPU), la mutación no se puede comprobar y hay que decirlo, no contarla
    como un fallo del test.
    """
    if informe is None:
        return True, ""
    proc = subprocess.run(informe, cwd=RAIZ, capture_output=True, text=True)
    if proc.returncode == 0:
        return True, ""
    salida = (proc.stdout + proc.stderr).strip().splitlines()
    return False, salida[-1] if salida else "el arnés no pudo correr"


def resumen(originales: dict[Path, str]) -> str:
    """Comprueba que los ficheros siguen byte a byte como estaban."""
    for ruta, contenido in originales.items():
        actual = ruta.read_text(encoding="utf-8")
        if actual != contenido:
            return f"el fichero {ruta} no quedó restaurado"
        resumen_hash = hashlib.sha256(actual.encode("utf-8")).hexdigest()[:12]
        print(f"  {ruta.relative_to(RAIZ)} restaurado (sha256 {resumen_hash})")
    return ""


def main() -> int:
    # Respaldo en memoria de todos los ficheros que se van a mutar y de los
    # informes que escriben los arneses.
    originales: dict[Path, str] = {}
    for _, relativo, _, _, _, _, _ in MUTACIONES:
        ruta = RAIZ / relativo
        if not ruta.exists():
            print(f"no existe {ruta}", file=sys.stderr)
            return 2
        if ruta not in originales:
            originales[ruta] = ruta.read_text(encoding="utf-8")
    for relativo in ARTEFACTOS:
        ruta = RAIZ / relativo
        if ruta.exists():
            originales[ruta] = ruta.read_text(encoding="utf-8")

    fallos = 0
    try:
        for nombre, relativo, antes, despues, objetivo, test, informe in MUTACIONES:
            ruta = RAIZ / relativo
            original = originales[ruta]
            if antes not in original:
                print(f"[{nombre}] el texto original no aparece: no se puede mutar")
                fallos += 1
                continue
            try:
                escribir(ruta, original.replace(antes, despues, 1))
                hecho, mensaje = preparar(informe)
                if not hecho:
                    print(f"[{nombre}] omitida: {mensaje}")
                    continue
                codigo, salida = ejecutar(objetivo, test)
                if codigo == 0:
                    print(f"[{nombre}] FALLO: el test {test} pasó con la mutación")
                    fallos += 1
                else:
                    print(f"[{nombre}] OK: el test {test} detectó la mutación")
            finally:
                # Restauración fiel del contenido original.
                escribir(ruta, original)
    finally:
        # Red de seguridad: si algo revienta a mitad, los ficheros vuelven.
        for ruta, contenido in originales.items():
            escribir(ruta, contenido)

    for nombre, _, _, _, objetivo, test, informe in MUTACIONES:
        hecho, mensaje = preparar(informe)
        if not hecho:
            print(f"[{nombre}] omitida al comprobar la restauración: {mensaje}")
            continue
        codigo, _ = ejecutar(objetivo, test)
        if codigo != 0:
            print(f"tras restaurar, el test {test} sigue fallando")
            fallos += 1

    problema = resumen(originales)
    if problema:
        print(problema)
        fallos += 1
    else:
        print("restauración verificada: los ficheros y los tests vuelven a su estado")

    return 1 if fallos else 0


if __name__ == "__main__":
    sys.exit(main())
