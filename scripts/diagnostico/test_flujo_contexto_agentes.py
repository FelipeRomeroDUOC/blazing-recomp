#!/usr/bin/env python3
"""
test_flujo_contexto_agentes.py — Suite de comprobación para la propuesta:
docs/propuestas/2026-11-06_inyeccion-contexto-fuentes-y-grafos.md (C1, C2 y C3).
"""

import json
import os
import sqlite3
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path

RAIZ = Path(__file__).resolve().parent.parent.parent
INDEXER = RAIZ / "scripts" / "indexar_contexto_y_fuentes.py"
SESSION_INIT = RAIZ / ".agents" / "hooks" / "session_init.py"
SOURCES_INDEX = RAIZ / ".agents" / "telemetry" / "sources_index.json"
GRAPH_JSON = RAIZ / "graphify-out" / "graph.json"
WIKI_INDEX = RAIZ / "graphify-out" / "wiki" / "index.md"
CODEGRAPH_DB = RAIZ / ".codegraph" / "codegraph.db"


class TestFlujoContextoAgentes(unittest.TestCase):
    def test_c1_indexador_y_grafos_limpios_con_fuentes_y_wgsl(self):
        self.assertTrue(INDEXER.exists(), "Debe existir scripts/indexar_contexto_y_fuentes.py")

        proc = subprocess.run(
            [sys.executable, str(INDEXER)],
            cwd=str(RAIZ),
            capture_output=True,
            text=True,
            encoding="utf-8",
        )
        self.assertEqual(proc.returncode, 0, f"Indexador falló: {proc.stderr} {proc.stdout}")

        # 1. Verificar sources_index.json
        self.assertTrue(SOURCES_INDEX.exists(), "Debe haberse generado .agents/telemetry/sources_index.json")
        idx = json.loads(SOURCES_INDEX.read_text(encoding="utf-8"))
        self.assertGreaterEqual(len(idx.get("fuentes", [])), 5, "Debe indexar los manuales de docs/fuentes/")
        self.assertGreaterEqual(len(idx.get("wgsl", {})), 2, "Debe indexar src/ppu_nativa.wgsl y src/overlay.wgsl")
        self.assertIn("src/ppu_nativa.wgsl", idx.get("wgsl", {}))
        self.assertTrue(
            any("fs_comp" in s.get("name", "") for s in idx["wgsl"]["src/ppu_nativa.wgsl"].get("symbols", [])),
            "Debe extraer fs_comp con su rango de líneas de src/ppu_nativa.wgsl",
        )

        # 2. Verificar graphify-out/graph.json y wiki/index.md sin docs/historico/
        self.assertTrue(GRAPH_JSON.exists(), "Debe existir graphify-out/graph.json")
        self.assertTrue(WIKI_INDEX.exists(), "Debe existir graphify-out/wiki/index.md")
        g_data = json.loads(GRAPH_JSON.read_text(encoding="utf-8"))
        g_nodes = g_data.get("nodes", [])
        hist_nodes = [n for n in g_nodes if "docs/historico" in (n.get("source_file") or "").replace("\\", "/")]
        self.assertEqual(len(hist_nodes), 0, f"graph.json no debe tener nodos de docs/historico: {hist_nodes[:3]}")

        md_nodes = [n for n in g_nodes if (n.get("source_file") or "").endswith(".md")]
        self.assertGreater(len(md_nodes), 10, "graph.json debe contener nodos documentales de las fuentes de verdad")

        wiki_txt = WIKI_INDEX.read_text(encoding="utf-8")
        self.assertNotIn("Community 0", wiki_txt, "wiki/index.md debe tener comunidades nombradas en español")
        self.assertTrue(
            "PPU" in wiki_txt and "Audio" in wiki_txt and "GBA" in wiki_txt,
            "wiki/index.md debe reflejar los dominios reales del proyecto",
        )

        # 3. Verificar .codegraph/codegraph.db sin docs/historico/
        if CODEGRAPH_DB.exists():
            conn = sqlite3.connect(str(CODEGRAPH_DB))
            hist_cg = conn.execute(
                "SELECT COUNT(*) FROM nodes WHERE REPLACE(file_path, '\\', '/') LIKE '%docs/historico/%'"
            ).fetchone()[0]
            conn.close()
            self.assertEqual(hist_cg, 0, "CodeGraph no debe tener nodos de docs/historico/")

        # 4. Verificar que docs/fuentes/ permanece intacto (R1)
        git_proc = subprocess.run(
            ["git", "status", "--porcelain", "docs/fuentes/"],
            cwd=str(RAIZ),
            capture_output=True,
            text=True,
            encoding="utf-8",
        )
        self.assertEqual(git_proc.stdout.strip(), "", "docs/fuentes/ es de solo lectura y no debe modificarse")

    def test_c2_session_init_inyecta_contexto_estatico_y_dinamico(self):
        # 1. Inicio de sesión del Orquestador Principal
        payload_main = {
            "conversationId": "main",
            "workspacePaths": [str(RAIZ)],
        }
        p_main = subprocess.run(
            [sys.executable, str(SESSION_INIT)],
            input=json.dumps(payload_main),
            capture_output=True,
            text=True,
            encoding="utf-8",
        )
        self.assertEqual(p_main.returncode, 0, f"session_init Orquestador falló: {p_main.stderr}")
        out_main = json.loads(p_main.stdout.strip())
        msg_main = out_main["injectSteps"][0]["systemMessage"]["systemMessage"]

        for esperado in (
            "Invariantes de Hardware GBA",
            "Zonas Sensibles",
            "ADR 33",
            "ADR 42",
            "Termómetro de Módulos src/",
            "registros-video-lcd-gba.md",
            "graphify-out/wiki/index.md",
        ):
            self.assertIn(esperado, msg_main, f"Falta '{esperado}' en la inyección del Orquestador")

        # 2. Inicio de sesión de un Subagente (Investigador) con prompt sobre BLDCNT y ppu_nativa.wgsl
        brain_sub_dir = Path(os.path.expanduser(r"~\.gemini\antigravity\brain\test-ctx-sub\.system_generated\subagents"))
        brain_sub_dir.mkdir(parents=True, exist_ok=True)
        sub_cid = "sub-ctx-test-999"
        sub_json = brain_sub_dir / f"{sub_cid}.json"
        try:
            sub_json.write_text(
                json.dumps(
                    {
                        "subagentDescriptor": {
                            "role": "Investigador",
                            "model": "flash",
                            "prompt": "Analiza BLDCNT y fs_comp en src/ppu_nativa.wgsl y revisa el blending de GBA",
                        }
                    }
                ),
                encoding="utf-8",
            )
            p_sub = subprocess.run(
                [sys.executable, str(SESSION_INIT)],
                input=json.dumps({"conversationId": sub_cid, "workspacePaths": [str(RAIZ)]}),
                capture_output=True,
                text=True,
                encoding="utf-8",
            )
            self.assertEqual(p_sub.returncode, 0, f"session_init Subagente falló: {p_sub.stderr}")
            out_sub = json.loads(p_sub.stdout.strip())
            msg_sub = out_sub["injectSteps"][0]["systemMessage"]["systemMessage"]

            self.assertIn("Subagente Activo: Investigador", msg_sub)
            self.assertIn("fs_comp", msg_sub, "Debe pre-inyectar la ubicación de fs_comp desde el índice WGSL")
            self.assertIn("registros-video-lcd-gba.md", msg_sub, "Debe pre-inyectar la fuente de verdad de registros LCD")
        finally:
            if sub_json.exists():
                sub_json.unlink()


if __name__ == "__main__":
    unittest.main()
