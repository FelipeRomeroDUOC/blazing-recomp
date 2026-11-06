#!/usr/bin/env python3
"""
scripts/diagnostico/test_comparar_hash_cuadros.py

Test suite para scripts/diagnostico/comparar_hash_cuadros.py.
Valida:
1. Coincidencia total (exit code 0).
2. Discrepancia en entrada 'ent' (exit code 1).
3. Discrepancia en salida 'sal' con 'ent' igual (exit code 1).
4. Faltante de datos / archivo vacio o no existente (exit code 2).
"""

import os
import subprocess
import sys
import tempfile

SCRIPT_COMPARADOR = os.path.join(
    os.path.dirname(__file__), "comparar_hash_cuadros.py"
)


def crear_linea(rel, frame, ent, sal, bld0="1111", bld50="2222", bld100="3333", bld150="4444", bg3=0):
    return f"# hash rel={rel} frame={frame} ent={ent} sal={sal} bld0={bld0} bld50={bld50} bld100={bld100} bld150={bld150} bg3_blend={bg3}\n"


def ejecutar(android_path, pc_path):
    cmd = [sys.executable, SCRIPT_COMPARADOR, "--android", android_path, "--pc", pc_path]
    res = subprocess.run(cmd, stdout=subprocess.PIPE, stderr=subprocess.PIPE, text=True)
    return res.returncode, res.stdout, res.stderr


def test_coincidencia_total():
    with tempfile.NamedTemporaryFile("w", delete=False) as f_a, tempfile.NamedTemporaryFile("w", delete=False) as f_p:
        f_a.write("ruido previo\n")
        f_p.write("otro ruido\n")
        for r in range(5):
            linea = crear_linea(r, 100 + r, f"aabb000{r}", f"ccdd000{r}")
            f_a.write(linea)
            f_p.write(linea)
        f_a.close()
        f_p.close()

        code, out, err = ejecutar(f_a.name, f_p.name)
        os.remove(f_a.name)
        os.remove(f_p.name)

        assert code == 0, f"Se esperaba 0, se obtuvo {code}. stderr: {err}"
        assert "Exito: Coincidencia total en 5 cuadros" in out, f"Salida inesperada: {out}"


def test_discrepancia_entrada():
    with tempfile.NamedTemporaryFile("w", delete=False) as f_a, tempfile.NamedTemporaryFile("w", delete=False) as f_p:
        f_a.write(crear_linea(0, 100, "1111", "aaaa"))
        f_p.write(crear_linea(0, 100, "1111", "aaaa"))
        # rel=1 difiere en ent
        f_a.write(crear_linea(1, 101, "2222", "bbbb"))
        f_p.write(crear_linea(1, 101, "9999", "bbbb"))
        f_a.close()
        f_p.close()

        code, out, err = ejecutar(f_a.name, f_p.name)
        os.remove(f_a.name)
        os.remove(f_p.name)

        assert code == 1, f"Se esperaba 1, se obtuvo {code}. stderr: {err}"
        assert "rel=1: ENTRADA difiere" in out, f"Mensaje incorrecto: {out}"


def test_discrepancia_salida():
    with tempfile.NamedTemporaryFile("w", delete=False) as f_a, tempfile.NamedTemporaryFile("w", delete=False) as f_p:
        f_a.write(crear_linea(0, 100, "1111", "aaaa"))
        f_p.write(crear_linea(0, 100, "1111", "aaaa"))
        # rel=1: ent igual, sal difiere
        f_a.write(crear_linea(1, 101, "2222", "bbbb"))
        f_p.write(crear_linea(1, 101, "2222", "ffff"))
        f_a.close()
        f_p.close()

        code, out, err = ejecutar(f_a.name, f_p.name)
        os.remove(f_a.name)
        os.remove(f_p.name)

        assert code == 1, f"Se esperaba 1, se obtuvo {code}. stderr: {err}"
        assert "rel=1: ENTRADA igual, SALIDA difiere" in out, f"Mensaje incorrecto: {out}"


def test_error_datos_faltantes():
    with tempfile.NamedTemporaryFile("w", delete=False) as f_a, tempfile.NamedTemporaryFile("w", delete=False) as f_p:
        f_a.write("sin lineas de hash\n")
        f_p.write(crear_linea(0, 100, "1111", "aaaa"))
        f_a.close()
        f_p.close()

        code, out, err = ejecutar(f_a.name, f_p.name)
        os.remove(f_a.name)
        os.remove(f_p.name)

        assert code == 2, f"Se esperaba 2, se obtuvo {code}. stdout: {out}"


if __name__ == "__main__":
    print("Ejecutando tests de comparar_hash_cuadros...")
    test_coincidencia_total()
    test_discrepancia_entrada()
    test_discrepancia_salida()
    test_error_datos_faltantes()
    print("Todos los tests pasaron exitosamente.")
