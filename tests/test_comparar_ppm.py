#!/usr/bin/env python3
"""
tests/test_comparar_ppm.py

Suite de pruebas unitarias para scripts/diagnostico/comparar_ppm.py.
Valida lectura de imágenes PPM P6, comparación de imágenes idénticas y con diferencias,
y generación de mapas de divergencia.
"""

import os
import sys
import tempfile
import unittest

# Asegurar que la raíz del repositorio esté en sys.path
sys.path.insert(0, os.path.abspath(os.path.join(os.path.dirname(__file__), "..")))

from scripts.diagnostico.comparar_ppm import (
    leer_ppm,
    leer_ppm_p6,
    comparar_ppm,
    generar_mapa_diferencias,
)


def escribir_ppm_p6(ruta: str, ancho: int, alto: int, pixeles: bytes, maxval: int = 255) -> None:
    encabezado = f"P6\n{ancho} {alto}\n{maxval}\n".encode("ascii")
    with open(ruta, "wb") as f:
        f.write(encabezado + pixeles)


class TestCompararPpm(unittest.TestCase):
    def setUp(self):
        self.temp_dir = tempfile.TemporaryDirectory()

    def tearDown(self):
        self.temp_dir.cleanup()

    def test_lectura_ppm_p6_sintetico(self):
        ruta = os.path.join(self.temp_dir.name, "sintetico.ppm")
        ancho, alto = 2, 2
        # 4 píxeles RGB: Rojo, Verde, Azul, Blanco
        datos = bytes([
            255, 0, 0,
            0, 255, 0,
            0, 0, 255,
            255, 255, 255,
        ])
        escribir_ppm_p6(ruta, ancho, alto, datos)

        w, h, maxval, pixeles = leer_ppm_p6(ruta)
        self.assertEqual(w, 2)
        self.assertEqual(h, 2)
        self.assertEqual(maxval, 255)
        self.assertEqual(pixeles, datos)

        # Verificar también con alias leer_ppm
        w2, h2, maxval2, pixeles2 = leer_ppm(ruta)
        self.assertEqual((w2, h2, maxval2, pixeles2), (w, h, maxval, pixeles))

    def test_comparacion_imagenes_identicas(self):
        ruta1 = os.path.join(self.temp_dir.name, "img1.ppm")
        ruta2 = os.path.join(self.temp_dir.name, "img2.ppm")
        ancho, alto = 2, 1
        datos = bytes([10, 20, 30, 40, 50, 60])
        escribir_ppm_p6(ruta1, ancho, alto, datos)
        escribir_ppm_p6(ruta2, ancho, alto, datos)

        codigo, metricas = comparar_ppm(ruta1, ruta2)
        self.assertEqual(codigo, 0)
        self.assertEqual(metricas["total_pixeles"], 2)
        self.assertEqual(metricas["identicos"], 2)
        self.assertEqual(metricas["distintos"], 0)
        self.assertEqual(metricas["pixeles_diferentes"], 0)
        self.assertEqual(metricas["pct_identicos"], 100.0)
        self.assertEqual(metricas["pct_distintos"], 0.0)
        self.assertEqual(metricas["max_dr"], 0)
        self.assertEqual(metricas["max_dg"], 0)
        self.assertEqual(metricas["max_db"], 0)
        self.assertEqual(metricas["delta_medio"], 0.0)

    def test_comparacion_imagenes_con_diferencias(self):
        ruta1 = os.path.join(self.temp_dir.name, "diff1.ppm")
        ruta2 = os.path.join(self.temp_dir.name, "diff2.ppm")
        ancho, alto = 2, 1
        # Píxel 0 idéntico, Píxel 1 diferente:
        # Píxel 1 en img1: (100, 150, 200)
        # Píxel 1 en img2: (110, 140, 230) -> dr=10, dg=10, db=30 -> delta_medio=(10+10+30)/3 = 50/3
        datos1 = bytes([10, 20, 30, 100, 150, 200])
        datos2 = bytes([10, 20, 30, 110, 140, 230])
        escribir_ppm_p6(ruta1, ancho, alto, datos1)
        escribir_ppm_p6(ruta2, ancho, alto, datos2)

        codigo, metricas = comparar_ppm(ruta1, ruta2)
        self.assertEqual(codigo, 1)
        self.assertEqual(metricas["total_pixeles"], 2)
        self.assertEqual(metricas["identicos"], 1)
        self.assertEqual(metricas["distintos"], 1)
        self.assertEqual(metricas["pixeles_diferentes"], 1)
        self.assertEqual(metricas["pct_identicos"], 50.0)
        self.assertEqual(metricas["pct_distintos"], 50.0)
        self.assertEqual(metricas["max_dr"], 10)
        self.assertEqual(metricas["max_dg"], 10)
        self.assertEqual(metricas["max_db"], 30)
        self.assertAlmostEqual(metricas["delta_medio"], 50.0 / 3.0, places=4)

    def test_generar_mapa_diferencias(self):
        ruta1 = os.path.join(self.temp_dir.name, "map1.ppm")
        ruta2 = os.path.join(self.temp_dir.name, "map2.ppm")
        ruta_diff = os.path.join(self.temp_dir.name, "diff_map.ppm")
        ancho, alto = 2, 1
        datos1 = bytes([0, 0, 0, 50, 50, 50])
        datos2 = bytes([0, 0, 0, 60, 60, 60])
        escribir_ppm_p6(ruta1, ancho, alto, datos1)
        escribir_ppm_p6(ruta2, ancho, alto, datos2)

        generar_mapa_diferencias(ruta1, ruta2, ruta_diff)
        self.assertTrue(os.path.exists(ruta_diff))

        w, h, maxval, pixeles_diff = leer_ppm_p6(ruta_diff)
        self.assertEqual(w, 2)
        self.assertEqual(h, 1)
        self.assertEqual(maxval, 255)
        # Píxel 0 idéntico -> (0, 0, 0)
        # Píxel 1 diferente -> magenta brillante (255, 0, 255)
        esperado = bytes([0, 0, 0, 255, 0, 255])
        self.assertEqual(pixeles_diff, esperado)


if __name__ == "__main__":
    unittest.main()
