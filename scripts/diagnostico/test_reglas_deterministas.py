#!/usr/bin/env python3
"""
test_reglas_deterministas.py — Pruebas de las reglas deterministas de agentes y habilidades
(Propuesta: docs/propuestas/2026-11-06_reglas-deterministas-agentes-skills.md).
"""

import json
import os
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path

RAIZ = Path(__file__).resolve().parent.parent.parent
GUARD = RAIZ / ".agents" / "hooks" / "pre_tool_guard.py"
VALIDATOR = RAIZ / "scripts" / "validar_propuesta_y_skills.py"
RUN_TESTS_SCRIPT = RAIZ / "scripts" / "run_tests_summary.py"
BUILD_APK_SCRIPT = RAIZ / "scripts" / "build_apk_summary.py"


def invoke_guard(tool_name: str, args: dict, cid: str = "test-det-guard") -> dict:
    payload = {
        "conversationId": cid,
        "stepIdx": 1,
        "workspacePaths": [str(RAIZ)],
        "toolCall": {"name": tool_name, "args": args},
    }
    p = subprocess.run(
        [sys.executable, str(GUARD)],
        input=json.dumps(payload),
        capture_output=True,
        text=True,
        encoding="utf-8",
    )
    assert p.returncode == 0, f"Guard falló con {p.returncode}: {p.stderr}"
    return json.loads(p.stdout.strip())


class TestReglasDeterministas(unittest.TestCase):
    def test_c1_bloquea_model_inherit_y_modelo_incorrecto(self):
        # 1. Prohibido Model='inherit'
        res = invoke_guard(
            "invoke_subagent",
            {"TypeName": "self", "Role": "Investigador", "Model": "inherit", "Prompt": "x"},
        )
        self.assertEqual(res.get("decision"), "deny", f"Debió bloquear inherit: {res}")
        self.assertIn("inherit", res.get("reason", "").lower())

        # 2. Prohibido asignar 'pro' a Constructor (debe ser 'flash')
        res_bad_role = invoke_guard(
            "invoke_subagent",
            {"TypeName": "self", "Role": "Constructor", "Model": "pro", "Prompt": "x"},
        )
        self.assertEqual(res_bad_role.get("decision"), "deny", f"Debió bloquear modelo erróneo: {res_bad_role}")

        # 3. Permitido asignar 'flash' a Investigador
        res_ok = invoke_guard(
            "invoke_subagent",
            {"TypeName": "self", "Role": "Investigador", "Model": "flash", "Prompt": "x"},
        )
        self.assertEqual(res_ok.get("decision"), "allow", f"Debió permitir combinación válida: {res_ok}")

    def test_c1_bloquea_debilitar_asserts_y_float_exacto(self):
        ruta_test = str(RAIZ / "src" / "audio_tests.rs")
        # 1. Eliminar un assert! en archivo de test
        res_assert = invoke_guard(
            "edit_file",
            {
                "Path": ruta_test,
                "Target": "assert_eq!(a, 1);\nassert_eq!(b, 2);",
                "Replacement": "assert_eq!(a, 1);",
            },
        )
        self.assertEqual(res_assert.get("decision"), "deny", f"Debió bloquear reducción de asserts: {res_assert}")

        # 2. Igualdad exacta f32 en assert_eq!
        res_float = invoke_guard(
            "edit_file",
            {
                "Path": ruta_test,
                "Target": "let x = 1.0;",
                "Replacement": "let x = 1.0;\nassert_eq!(distancia, 3.1415_f32);",
            },
        )
        self.assertEqual(res_float.get("decision"), "deny", f"Debió bloquear assert_eq! f32 exacto: {res_float}")

    def test_c1_bloquea_modulo_excesivo_con_tests_inline(self):
        ruta_mod = str(RAIZ / "src" / "nuevo_modulo.rs")
        codigo_largo = "\n".join(["pub fn f() {}"] * 360) + "\n#[cfg(test)]\nmod tests {\n    #[test]\n    fn t() {}\n}\n"
        res = invoke_guard(
            "write_file",
            {"Path": ruta_mod, "Content": codigo_largo},
        )
        self.assertEqual(res.get("decision"), "deny", f"Debió bloquear módulo >350L con #[cfg(test)] inline: {res}")

    def test_c2_validador_propuestas_y_skills(self):
        self.assertTrue(VALIDATOR.exists(), "Debe existir scripts/validar_propuesta_y_skills.py")

        with tempfile.TemporaryDirectory() as tmp:
            bad_prop = Path(tmp) / "2026-11-06_mala.md"
            bad_prop.write_text(
                "# Propuesta: [nombre corto]\n\nEstado: pendiente\n\n## Revisión humana\n- algo: ✔\n",
                encoding="utf-8",
            )
            p_bad = subprocess.run(
                [sys.executable, str(VALIDATOR), str(bad_prop)],
                capture_output=True,
                text=True,
                encoding="utf-8",
            )
            self.assertNotEqual(p_bad.returncode, 0, "Propuesta con plantilla sin rellenar debe fallar")

            good_prop = RAIZ / "docs" / "propuestas" / "2026-11-06_reglas-deterministas-agentes-skills.md"
            p_good = subprocess.run(
                [sys.executable, str(VALIDATOR), str(good_prop)],
                capture_output=True,
                text=True,
                encoding="utf-8",
            )
            self.assertEqual(p_good.returncode, 0, f"Propuesta válida debe pasar: {p_good.stdout} {p_good.stderr}")

    def test_c3_scripts_reemplazo_y_subagentes_podados(self):
        self.assertTrue(RUN_TESTS_SCRIPT.exists(), "Debe existir scripts/run_tests_summary.py")
        self.assertTrue(BUILD_APK_SCRIPT.exists(), "Debe existir scripts/build_apk_summary.py")
        self.assertFalse(
            (RAIZ / ".agents" / "subagents" / "test-runner.md").exists(),
            "test-runner.md debe haber sido reemplazado por script",
        )
        self.assertFalse(
            (RAIZ / ".agents" / "subagents" / "apk-builder.md").exists(),
            "apk-builder.md debe haber sido reemplazado por script",
        )

    def test_c4_antibypass_orquestador_lecturas_y_python_c(self):
        # 1. Bloquear python -c en el Orquestador (evasión de read_file)
        res_pyc = invoke_guard(
            "run_command",
            {"CommandLine": "python -c \"print(open('src/ppu_nativa.rs').read())\""},
            cid="main-orch-session",
        )
        self.assertEqual(res_pyc.get("decision"), "deny", f"Debió bloquear python -c en Orquestador: {res_pyc}")

        # 2. Bloquear Get-Content / cat / type en el Orquestador
        res_cat = invoke_guard(
            "run_command",
            {"CommandLine": "Get-Content telemetria/blazing-perf.txt -Head 50"},
            cid="main-orch-session",
        )
        self.assertEqual(res_cat.get("decision"), "deny", f"Debió bloquear Get-Content en Orquestador: {res_cat}")

        # 3. Bloquear lectura directa de telemetria / logs / ppm con read_file en el Orquestador
        res_tel = invoke_guard(
            "read_file",
            {"Path": str(RAIZ / "telemetria" / "blazing-perf.txt")},
            cid="main-orch-session",
        )
        self.assertEqual(res_tel.get("decision"), "deny", f"Debió bloquear lectura de telemetría en Orquestador: {res_tel}")

        res_cuadro = invoke_guard(
            "read_file",
            {"Path": str(RAIZ / "telemetria" / "cuadro-150-pc.ppm")},
            cid="main-orch-session",
        )
        self.assertEqual(res_cuadro.get("decision"), "deny", f"Debió bloquear lectura de cuadro-* en Orquestador: {res_cuadro}")


if __name__ == "__main__":
    unittest.main()
