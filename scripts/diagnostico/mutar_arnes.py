#!/usr/bin/env python3
"""Mutación del arnés común de los experimentos, para comprobar que los tests la detectan.

El script:
  1. lee el contenido original del fichero mutado en memoria,
  2. aplica una mutación (un cambio concreto),
  3. ejecuta el test de aceptación y comprueba que **falla** (es decir, que el test
     detecta el fallo),
  4. restaura el contenido original en un bloque `finally`,
  5. vuelve a ejecutar el test y comprueba que vuelve a pasar.

No usa comandos destructivos de Git: la restauración es en memoria y sobre el disco
directamente, para no perder trabajo sin commitear.

Uso:
    python scripts/diagnostico/mutar_arnes.py
"""

from __future__ import annotations

import subprocess
import sys
from pathlib import Path

RAIZ = Path(__file__).resolve().parents[2]

# (título, fichero, texto original, texto mutado, test que tiene que morir)
MUTACIONES = [
    (
        "un experimento vuelve a declarar su propio arnés",
        "examples/ppu_capas.rs",
        '#[path = "comun/mod.rs"]\nmod comun;\n',
        "",
        "test_los_experimentos_usan_el_modulo_comun",
    ),
    (
        "un experimento recupera su copia de leer_secuencia",
        "examples/ppu_estado.rs",
        '#[path = "comun/mod.rs"]\nmod comun;',
        '#[path = "comun/mod.rs"]\nmod comun;\n'
        "fn leer_secuencia(texto: &str) -> Result<Vec<(u32, u16)>, String> "
        "{ let _ = texto; Ok(Vec::new()) }",
        "test_los_experimentos_usan_el_modulo_comun",
    ),
    (
        "el parser deja de rechazar un botón desconocido",
        "examples/comun/entrada.rs",
        ".ok_or_else(|| format!(\"línea {}: botón desconocido '{}'\", n + 1, boton))?;",
        ".unwrap_or(0);",
        "test_la_secuencia_rechaza_un_boton_desconocido",
    ),
    (
        "el parser deja de ignorar los comentarios",
        "examples/comun/entrada.rs",
        "let linea = linea.split('#').next().unwrap_or(\"\").trim();",
        "let linea = linea.trim();",
        "test_la_secuencia_se_lee_igual_que_antes",
    ),
    (
        "el orden de bits de los botones cambia",
        "examples/comun/entrada.rs",
        '"A", "B", "Select", "Start", "Right", "Left", "Up", "Down", "R", "L",',
        '"B", "A", "Select", "Start", "Right", "Left", "Up", "Down", "R", "L",',
        "test_la_secuencia_se_lee_igual_que_antes",
    ),
]


def correr_test(test: str) -> subprocess.CompletedProcess:
    return subprocess.run(
        ["cargo", "test", "--test", "arnes_experimentos_tests", test],
        cwd=RAIZ,
        capture_output=True,
        text=True,
    )


def main() -> int:
    fallos = 0
    for titulo, ruta_rel, viejo, nuevo, test in MUTACIONES:
        ruta = RAIZ / ruta_rel
        original = ruta.read_text(encoding="utf-8")
        if viejo not in original:
            print(f"NO ENCONTRADO en {ruta_rel}: {viejo[:40]!r}")
            fallos += 1
            continue
        try:
            ruta.write_text(original.replace(viejo, nuevo, 1), encoding="utf-8", newline="\n")
            proc = correr_test(test)
            murio = proc.returncode != 0
            print(f"{'MUERTA  ' if murio else 'VIVE   '} {titulo}  ->  {test}")
            if not murio:
                fallos += 1
        finally:
            ruta.write_text(original, encoding="utf-8", newline="\n")

        # Con el fichero restaurado, el test tiene que volver a pasar.
        if correr_test(test).returncode != 0:
            print(f"NO VUELVE A PASAR tras restaurar: {test}")
            fallos += 1

    muertas = len(MUTACIONES) - fallos
    print(f"\n{muertas} de {len(MUTACIONES)} mutaciones muertas")
    return 1 if fallos else 0


if __name__ == "__main__":
    sys.exit(main())
