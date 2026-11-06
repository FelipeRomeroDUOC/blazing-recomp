#!/usr/bin/env python3
"""
scripts/diagnostico/test_comparar_hash_cuadros_componentes.py

Pruebas de los escenarios de la propuesta
`2026-11-02_desglosar-hash-y-medir-salida-gpu-dragon`: el script debe nombrar
el componente de entrada que difiere y no comparar la salida si las entradas
son distintas. No depende de pytest.
"""

import os
import subprocess
import sys
import tempfile

AQUI = os.path.dirname(os.path.abspath(__file__))
SCRIPT = os.path.join(AQUI, "comparar_hash_cuadros.py")

BASE = (
    "# hash rel={rel} frame={rel} ent={ent} sal={sal} bld0=2168 bld50=2168 "
    "bld100=2168 bld150=2168 bg3_blend=160"
)
EXTRA = " hc={hc} hv={hv} ho={ho} hp={hp} hl={hl}"


def linea(rel, ent="AAAA0001", sal="BBBB0001", hc="01", hv="02", ho="03", hp="04", hl="05", con_extra=True):
    texto = BASE.format(rel=rel, ent=ent, sal=sal)
    if con_extra:
        texto += EXTRA.format(hc=hc, hv=hv, ho=ho, hp=hp, hl=hl)
    return texto + "\n"


def ejecutar(lineas_android, lineas_pc):
    with tempfile.TemporaryDirectory() as d:
        a = os.path.join(d, "a.txt")
        p = os.path.join(d, "p.txt")
        open(a, "w", encoding="utf-8").write("".join(lineas_android))
        open(p, "w", encoding="utf-8").write("".join(lineas_pc))
        r = subprocess.run(
            [sys.executable, SCRIPT, "--android", a, "--pc", p],
            capture_output=True,
            text=True,
        )
        return r.returncode, (r.stdout + r.stderr)


def test_nombra_componente_vram():
    a = [linea(0), linea(1, ent="AAAA0002", hv="02")]
    p = [linea(0), linea(1, ent="CCCC0002", hv="99")]
    codigo, salida = ejecutar(a, p)
    assert codigo == 1, (codigo, salida)
    assert "rel=1: ENTRADA difiere en: VRAM" in salida, salida
    assert "SALIDA difiere" not in salida, salida


def test_nombra_paleta_por_linea():
    a = [linea(0, hl="05")]
    p = [linea(0, ent="CCCC0001", hl="77")]
    codigo, salida = ejecutar(a, p)
    assert codigo == 1, (codigo, salida)
    assert "paleta_lineas" in salida, salida


def test_entradas_iguales_salida_difiere():
    a = [linea(0), linea(1)]
    p = [linea(0), linea(1, sal="DDDD0001")]
    codigo, salida = ejecutar(a, p)
    assert codigo == 1, (codigo, salida)
    assert "rel=1: ENTRADAS iguales, SALIDA difiere" in salida, salida


def test_formato_antiguo_sigue_valido():
    a = [linea(0, con_extra=False), linea(1, con_extra=False)]
    p = [linea(0, con_extra=False), linea(1, con_extra=False)]
    codigo, salida = ejecutar(a, p)
    assert codigo == 0, (codigo, salida)


def test_formato_antiguo_ent_distinta_sigue_igual_que_antes():
    a = [linea(0, con_extra=False)]
    p = [linea(0, ent="CCCC0001", con_extra=False)]
    codigo, salida = ejecutar(a, p)
    assert codigo == 1, (codigo, salida)
    assert "ENTRADA difiere" in salida, salida


if __name__ == "__main__":
    pruebas = [v for k, v in sorted(globals().items()) if k.startswith("test_")]
    fallos = 0
    for prueba in pruebas:
        try:
            prueba()
            print(f"OK   {prueba.__name__}")
        except AssertionError as e:
            fallos += 1
            print(f"FALLA {prueba.__name__}: {e}")
    print(f"{len(pruebas) - fallos}/{len(pruebas)} pasan")
    sys.exit(1 if fallos else 0)
