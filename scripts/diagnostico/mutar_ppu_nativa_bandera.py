#!/usr/bin/env python3
"""Mutación controlada de la marca de runtime de la PPU nativa y de su telemetría.

El script:
  1. lee los ficheros originales en memoria,
  2. aplica una mutación (un cambio concreto del interruptor, del guardián de
     errores o del contador de telemetría),
  3. ejecuta el test que debe detectarla,
  4. restaura el contenido original en un bloque `finally`,
  5. comprueba que el test falló con la mutación (es decir, que el test
     detecta el fallo) y que vuelve a pasar sin ella.

No usa comandos destructivos de Git: la restauración es en memoria y sobre el
disco directamente, para no perder trabajo sin commitear.

Uso:
  python scripts/diagnostico/mutar_ppu_nativa_bandera.py
"""

from __future__ import annotations

import subprocess
import sys
from pathlib import Path

RAIZ = Path(__file__).resolve().parents[2]

# (fichero, nombre, texto original, texto mutado, test que debe detectarlo)
MUTACIONES = [
    (
        "src/ppu_nativa.rs",
        "marca_siempre_apagada",
        "        .any(|dir| dir.join(NOMBRE_BANDERA_APAGADO).exists())",
        "        .any(|dir| true || dir.join(NOMBRE_BANDERA_APAGADO).exists())",
        "ppu_nativa::tests::sin_marca_la_ppu_nativa_esta_activa",
    ),
    (
        "src/ppu_nativa.rs",
        "marca_siempre_encendida",
        "        .any(|dir| dir.join(NOMBRE_BANDERA_APAGADO).exists())",
        "        .any(|dir| false && dir.join(NOMBRE_BANDERA_APAGADO).exists())",
        "ppu_nativa::tests::la_marca_de_apagado_apaga_la_ppu_nativa",
    ),
    (
        "src/ppu_nativa.rs",
        "nombre_de_archivo_distinto",
        'pub const NOMBRE_BANDERA_APAGADO: &str = "ppu-nativa-off.flag";',
        'pub const NOMBRE_BANDERA_APAGADO: &str = "ppu-nativa.txt";',
        "ppu_nativa::tests::el_nombre_de_la_marca_de_apagado_es_el_documentado",
    ),
    (
        "src/ppu_nativa.rs",
        "guardian_sin_ambito_de_error",
        "    for filtro in FILTROS_ERROR {\n        motor.abrir_ambito(filtro);\n    }",
        "    for filtro in FILTROS_ERROR {\n        let _ = filtro;\n    }",
        "ppu_nativa::tests::el_guardian_devuelve_el_error_en_vez_de_dejar_la_ppu_nativa_a_medias",
    ),
    (
        "src/ppu_nativa.rs",
        "guardian_ignora_el_error",
        "    if errores.is_empty() {\n        Ok(valor)\n    } else {\n        Err(format!(\"{etiqueta}: {}\", errores.join(\"; \")))\n    }",
        "    if errores.is_empty() {\n        Ok(valor)\n    } else {\n        Ok(valor)\n    }",
        "ppu_nativa::tests::el_guardian_devuelve_el_error_en_vez_de_dejar_la_ppu_nativa_a_medias",
    ),
    (
        "src/perf.rs",
        "telemetria_sin_contar_la_ppu_nativa",
        "        if sample.ppu_nativa {\n            self.hw_nativa_frames += 1;\n        }",
        "        if false && sample.ppu_nativa {\n            self.hw_nativa_frames += 1;\n        }",
        "perf::tests::test_ppu_hw_distingue_los_fotogramas_de_la_ppu_nativa",
    ),
]


def ejecutar(test: str) -> tuple[int, str]:
    proc = subprocess.run(
        ["cargo", "test", "--lib", test, "--", "--exact"],
        cwd=RAIZ,
        capture_output=True,
        text=True,
    )
    return proc.returncode, proc.stdout + proc.stderr


def main() -> int:
    originales: dict[Path, str] = {}
    for relativo, *_ in MUTACIONES:
        ruta = RAIZ / relativo
        if not ruta.exists():
            print(f"no existe {ruta}", file=sys.stderr)
            return 2
        originales.setdefault(ruta, ruta.read_text(encoding="utf-8"))

    fallos = 0

    for relativo, nombre, antes, despues, test in MUTACIONES:
        ruta = RAIZ / relativo
        original = originales[ruta]
        if antes not in original:
            print(f"[{nombre}] el texto original no aparece: no se puede mutar")
            fallos += 1
            continue
        try:
            ruta.write_text(original.replace(antes, despues, 1), encoding="utf-8")
            codigo, salida = ejecutar(test)
            if codigo == 0:
                print(f"[{nombre}] FALLO: el test {test} pasó con la mutación")
                fallos += 1
            else:
                print(f"[{nombre}] OK: el test {test} detectó la mutación")
        finally:
            # Restauración fiel del contenido original.
            ruta.write_text(original, encoding="utf-8")

    codigo, _ = ejecutar("ppu_nativa::tests::la_marca_de_apagado_apaga_la_ppu_nativa")
    if codigo != 0:
        print("el test no vuelve a pasar tras restaurar los ficheros")
        fallos += 1
    else:
        print("restauración verificada: los tests vuelven a pasar")

    return 1 if fallos else 0


if __name__ == "__main__":
    sys.exit(main())
