#!/usr/bin/env python3
"""
indexar_contexto_y_fuentes.py — Indexador determinista de Contexto, Fuentes de Verdad,
Shaders WGSL, CodeGraph y Graphify para Blazing Recomp.

Responsabilidades:
1. Auto-descubre todos los manuales y referencias en `docs/fuentes/`, `docs/referencia/`,
   `docs/explicacion/` y `GLOSSARY.md`, extrayendo títulos, palabras clave y el mapa de
   secciones `##` con sus rangos exactos de líneas (`L1-L2`).
2. Extrae de `docs/explicacion/DECISIONES.md` los ADRs vigentes recientes y los ADRs con
   resultado negativo / prohibidos.
3. Parsea los shaders WGSL (`src/*.wgsl`) extrayendo `struct`, `fn` y `@binding` con sus
   rangos de líneas (`L1-L2`).
4. Calcula el termómetro de líneas de código vs. tests inline para todos los archivos en `src/*.rs`.
5. Purga de `.codegraph/codegraph.db` cualquier archivo o nodo bajo `docs/historico/`.
6. Limpia `graphify-out/graph.json` de código muerto (`docs/historico/`), inyecta los nodos
   documentales (`docs/fuentes/`, `docs/referencia/`, `docs/explicacion/`, `GLOSSARY.md`) y
   los símbolos WGSL, asigna nombres de dominio en español a las comunidades y regenera
   `graphify-out/GRAPH_REPORT.md` y `graphify-out/wiki/index.md` (+ artículos por comunidad).
"""

from __future__ import annotations

import json
import os
import re
import sqlite3
import subprocess
import sys
import time
from collections import Counter
from datetime import datetime
from pathlib import Path

RAIZ = Path(__file__).resolve().parent.parent
TELEMETRY_DIR = RAIZ / ".agents" / "telemetry"
SOURCES_INDEX_PATH = TELEMETRY_DIR / "sources_index.json"
GRAPHIFY_DIR = RAIZ / "graphify-out"
CODEGRAPH_DB = RAIZ / ".codegraph" / "codegraph.db"

DOMAIN_COMMUNITIES = {
    0: ("Hardware GBA, Memoria y Buses", "Especificaciones del silicio ARM7TDMI, IWRAM/EWRAM, DMA, timers y validación SHA-256 de ROM/BIOS."),
    1: ("PPU Nativa y Shaders WGSL", "Pipeline gráfico en GPU (`src/ppu_nativa.rs`, `src/ppu_nativa.wgsl`), capas BG0-BG3, sprites OBJ, ventanas y mezcla alfa (`BLDCNT`)."),
    2: ("GPUs Móviles TBDR, Presentación y Swapchain", "Arquitectura de GPUs móviles TBDR/UMA, texturas R8, subida diferencial en VBlank, `SurfaceView` y calibración de swapchain (`src/present.rs`)."),
    3: ("Audio Nativo AAudio y Motor MP2K/M4A", "Motor de sonido de baja latencia con AAudio NDK (`src/audio.rs`), remuestreo DRC, cola circular SPSC lock-free y mezclador MP2K HLE."),
    4: ("Recompilación Estática AOT C11 y Motor FE7", "Bloques C11 (`src/recomp.rs`, `blazing.labels.toml`), anatomía del motor de Fire Emblem 7, rutinas en IWRAM y despacho de VBlank."),
    5: ("Plataforma Android, Controles y Persistencia", "Ciclo de vida `NativeActivity` (`src/platform.rs`), controles táctiles SDF (`src/input.rs`, `src/overlay.rs`) y guardado `.sav` / SAF."),
    6: ("Arnés de Agentes, Telemetría y Pruebas", "Hooks deterministas (`.agents/hooks/`), registro SQLite (`ledger.py`), scripts de diagnóstico y suites de pruebas."),
}


def slugify_id(text: str) -> str:
    clean = re.sub(r"[^a-zA-Z0-9]+", "_", text.lower()).strip("_")
    return clean[:80] or "node"


def classify_domain_community(rel_path: str, label: str = "") -> int:
    p = (rel_path or "").replace("\\", "/").lower()
    lbl = (label or "").lower()
    combined = f"{p} {lbl}"
    if any(k in combined for k in ("ppu_nativa", "registros-video", "bldcnt", "dispcnt", "fs_comp", "fs_bg", "ppu-caminos")):
        return 1
    if any(k in combined for k in ("tbdr", "present", "swapchain", "surfaceview", "calibr")):
        return 2
    if any(k in combined for k in ("audio", "aaudio", "mp2k", "m4a", "resampl", "ringbuffer")):
        return 3
    if any(k in combined for k in ("recomp", "labels", "anatomia-motor", "c-recompilado", "pack.toml", "iwram", "aarch32")):
        return 4
    if any(k in combined for k in ("platform", "input", "overlay", "almacenamiento", "saf", "error_screen", "apk")):
        return 5
    if any(k in combined for k in ("manual técnico", "gba", "runner", "pipeline", "assets", "sha256", "bios")):
        return 0
    return 6


def parse_markdown_sections(md_path: Path) -> dict:
    rel = md_path.relative_to(RAIZ).as_posix()
    text = md_path.read_text(encoding="utf-8", errors="replace")
    lines = text.splitlines()
    total_lines = len(lines)

    title = md_path.stem
    for ln in lines[:30]:
        m_h1 = re.match(r"^#\s+(.+)$", ln.strip())
        if m_h1:
            title = m_h1.group(1).strip()
            break

    sections = []
    current_sec = None
    for idx, ln in enumerate(lines, start=1):
        m_h2 = re.match(r"^##\s+(.+)$", ln.strip())
        if m_h2:
            if current_sec is not None:
                current_sec["end_line"] = idx - 1
                sections.append(current_sec)
            sec_title = re.sub(r"\[([^\]]+)\]\([^)]+\)", r"\1", m_h2.group(1).strip())
            current_sec = {
                "title": sec_title,
                "start_line": idx,
                "end_line": total_lines,
            }
    if current_sec is not None:
        current_sec["end_line"] = total_lines
        sections.append(current_sec)

    # Extraer palabras clave relevantes del título y encabezados
    kw_raw = f"{title} " + " ".join(s["title"] for s in sections)
    words = {
        w.lower()
        for w in re.findall(r"[A-Za-z0-9_áéíóúÁÉÍÓÚñÑ]{3,}", kw_raw)
        if w.lower() not in {"para", "con", "del", "las", "los", "una", "por", "como", "que", "sobre", "entre"}
    }

    return {
        "path": rel,
        "title": title,
        "lines": total_lines,
        "sections": sections[:15],
        "keywords": sorted(words)[:30],
    }


def extract_adrs_from_decisiones() -> dict:
    dec_path = RAIZ / "docs" / "explicacion" / "DECISIONES.md"
    if not dec_path.exists():
        return {"negativos": [], "vigentes_recientes": [], "total": 0}

    text = dec_path.read_text(encoding="utf-8", errors="replace")
    toc_pattern = re.compile(
        r"^(\d+)\.\s+\[([^\]]+)\]\(#[^)]+\)\s+—\s+\*\*\[([^\]]+)\]\*\*",
        re.M,
    )
    negativos = []
    vigentes = []
    all_adrs = []

    for m in toc_pattern.finditer(text):
        num = int(m.group(1))
        titulo = m.group(2).strip()
        estado = m.group(3).strip()
        item = {"num": num, "title": titulo, "status": estado}
        all_adrs.append(item)
        est_low = estado.lower()
        if "negativo" in est_low or num == 42:
            negativos.append(item)
        elif "vigente" in est_low and "sustituida" not in est_low and "histórica" not in est_low:
            vigentes.append(item)

    return {
        "total": len(all_adrs),
        "negativos": negativos,
        "vigentes_recientes": vigentes[-8:],
    }


def parse_wgsl_shaders() -> dict[str, dict]:
    wgsl_map: dict[str, dict] = {}
    for wgsl_path in sorted((RAIZ / "src").glob("*.wgsl")):
        rel = wgsl_path.relative_to(RAIZ).as_posix()
        lines = wgsl_path.read_text(encoding="utf-8", errors="replace").splitlines()
        total_lines = len(lines)
        symbols = []

        for idx, ln in enumerate(lines, start=1):
            s = ln.strip()
            m_fn = re.match(r"^(?:@\w+(?:\([^)]*\))?\s*)*fn\s+([A-Za-z0-9_]+)\s*\(", s)
            m_st = re.match(r"^struct\s+([A-Za-z0-9_]+)\s*\{?", s)
            if m_fn or m_st:
                kind = "fn" if m_fn else "struct"
                name = (m_fn or m_st).group(1)
                start_l = idx
                # Buscar cierre de bloque aproximado por balance de llaves
                brace_depth = 0
                opened = False
                end_l = idx
                for j in range(idx - 1, min(total_lines, idx + 250)):
                    c_open = lines[j].count("{")
                    c_close = lines[j].count("}")
                    if c_open > 0:
                        opened = True
                    brace_depth += c_open - c_close
                    if opened and brace_depth <= 0:
                        end_l = j + 1
                        break
                symbols.append(
                    {
                        "name": name,
                        "kind": kind,
                        "start_line": start_l,
                        "end_line": end_l,
                    }
                )

        wgsl_map[rel] = {
            "path": rel,
            "lines": total_lines,
            "symbols": symbols,
        }
    return wgsl_map


def scan_rust_src_thermometer() -> list[dict]:
    mods = []
    src_dir = RAIZ / "src"
    if not src_dir.exists():
        return mods

    for rs_path in sorted(src_dir.glob("*.rs")):
        if rs_path.name.endswith("_tests.rs"):
            continue
        lines = rs_path.read_text(encoding="utf-8", errors="replace").splitlines()
        total = len(lines)
        cfg_idx = next(
            (i for i, ln in enumerate(lines) if re.search(r"#\s*\[\s*cfg\s*\(\s*test\s*\)\s*\]", ln)),
            None,
        )
        code_lines = cfg_idx if cfg_idx is not None else total
        inline_test_lines = (total - cfg_idx) if cfg_idx is not None else 0
        sat_name = f"{rs_path.stem}_tests.rs"
        has_satellite = (src_dir / sat_name).exists()
        mods.append(
            {
                "file": f"src/{rs_path.name}",
                "total_lines": total,
                "code_lines": code_lines,
                "inline_test_lines": inline_test_lines,
                "has_satellite": has_satellite,
            }
        )
    return mods


def clean_codegraph_db() -> int:
    if not CODEGRAPH_DB.exists():
        return 0
    try:
        conn = sqlite3.connect(str(CODEGRAPH_DB), timeout=5.0)
        with conn:
            bad_ids = [
                r[0]
                for r in conn.execute(
                    "SELECT id FROM nodes WHERE REPLACE(file_path, '\\', '/') LIKE '%docs/historico/%' OR REPLACE(file_path, '\\', '/') LIKE '%.cloud-attachments/%'"
                ).fetchall()
            ]
            if bad_ids:
                conn.execute(
                    "DELETE FROM edges WHERE source IN (SELECT id FROM nodes WHERE REPLACE(file_path, '\\', '/') LIKE '%docs/historico/%') "
                    "OR target IN (SELECT id FROM nodes WHERE REPLACE(file_path, '\\', '/') LIKE '%docs/historico/%')"
                )
                conn.execute(
                    "DELETE FROM nodes WHERE REPLACE(file_path, '\\', '/') LIKE '%docs/historico/%' OR REPLACE(file_path, '\\', '/') LIKE '%.cloud-attachments/%'"
                )
            conn.execute(
                "DELETE FROM files WHERE REPLACE(path, '\\', '/') LIKE '%docs/historico/%' OR REPLACE(path, '\\', '/') LIKE '%.cloud-attachments/%'"
            )
            conn.execute(
                "DELETE FROM unresolved_refs WHERE REPLACE(file_path, '\\', '/') LIKE '%docs/historico/%'"
            )
        conn.close()
        return len(bad_ids)
    except Exception:
        return 0


def update_graphify_and_wiki(
    fuentes_docs: list[dict],
    ref_docs: list[dict],
    exp_docs: list[dict],
    wgsl_map: dict[str, dict],
) -> dict:
    GRAPHIFY_DIR.mkdir(parents=True, exist_ok=True)
    graph_path = GRAPHIFY_DIR / "graph.json"
    manifest_path = GRAPHIFY_DIR / "manifest.json"
    report_path = GRAPHIFY_DIR / "GRAPH_REPORT.md"
    wiki_dir = GRAPHIFY_DIR / "wiki"
    wiki_dir.mkdir(parents=True, exist_ok=True)

    if graph_path.exists():
        try:
            g_data = json.loads(graph_path.read_text(encoding="utf-8"))
        except Exception:
            g_data = {"directed": False, "multigraph": False, "graph": {}, "nodes": [], "links": [], "hyperedges": []}
    else:
        g_data = {"directed": False, "multigraph": False, "graph": {}, "nodes": [], "links": [], "hyperedges": []}

    def is_excluded(sf: str) -> bool:
        norm = (sf or "").replace("\\", "/").lower()
        return any(
            bad in norm
            for bad in ("docs/historico/", ".cloud-attachments/", "/evals/", "test_reglas_deterministas")
        )

    # 1. Filtrar nodos AST existentes eliminando docs/historico/ y nodos semánticos viejos para regenerarlos limpios
    clean_nodes = []
    valid_ids = set()
    for n in g_data.get("nodes", []):
        sf = (n.get("source_file") or "").replace("\\", "/")
        if is_excluded(sf):
            continue
        if n.get("_origin") == "semantic" or sf.endswith((".md", ".wgsl")):
            continue
        cid = classify_domain_community(sf, n.get("label", ""))
        n["community"] = cid
        n["community_name"] = DOMAIN_COMMUNITIES[cid][0]
        n["source_file"] = sf
        clean_nodes.append(n)
        valid_ids.add(n["id"])

    clean_links = [
        e
        for e in g_data.get("links", [])
        if e.get("source") in valid_ids
        and e.get("target") in valid_ids
        and not is_excluded(e.get("source_file", ""))
    ]

    # 2. Agregar nodos y enlaces de Shaders WGSL
    for rel_wgsl, winfo in wgsl_map.items():
        mod_id = slugify_id(rel_wgsl)
        cid = 1  # PPU Nativa y Shaders WGSL
        if mod_id not in valid_ids:
            clean_nodes.append(
                {
                    "id": mod_id,
                    "label": Path(rel_wgsl).name,
                    "_origin": "ast",
                    "community": cid,
                    "community_name": DOMAIN_COMMUNITIES[cid][0],
                    "file_type": "code",
                    "norm_label": Path(rel_wgsl).stem.lower(),
                    "source_file": rel_wgsl,
                    "source_location": f"L1-L{winfo['lines']}",
                }
            )
            valid_ids.add(mod_id)

        for sym in winfo.get("symbols", []):
            sym_id = slugify_id(f"{rel_wgsl}_{sym['name']}")
            if sym_id not in valid_ids:
                clean_nodes.append(
                    {
                        "id": sym_id,
                        "label": f"{sym['name']}()",
                        "_callable": sym["kind"] == "fn",
                        "_origin": "ast",
                        "community": cid,
                        "community_name": DOMAIN_COMMUNITIES[cid][0],
                        "file_type": "code",
                        "norm_label": sym["name"].lower(),
                        "source_file": rel_wgsl,
                        "source_location": f"L{sym['start_line']}-L{sym['end_line']}",
                    }
                )
                valid_ids.add(sym_id)
                clean_links.append(
                    {
                        "source": mod_id,
                        "target": sym_id,
                        "relation": "contains",
                        "_origin": "ast",
                        "confidence": "EXTRACTED",
                        "confidence_score": 1.0,
                        "source_file": rel_wgsl,
                        "source_location": f"L{sym['start_line']}",
                        "weight": 1.0,
                    }
                )

    # 3. Agregar nodos documentales de las Fuentes de Verdad, Referencia y Explicación
    all_docs = fuentes_docs + ref_docs + exp_docs
    for d in all_docs:
        rel_doc = d["path"]
        doc_id = slugify_id(rel_doc)
        cid = classify_domain_community(rel_doc, d["title"])
        if doc_id not in valid_ids:
            clean_nodes.append(
                {
                    "id": doc_id,
                    "label": d["title"],
                    "_origin": "semantic",
                    "community": cid,
                    "community_name": DOMAIN_COMMUNITIES[cid][0],
                    "file_type": "document",
                    "norm_label": d["title"].lower(),
                    "source_file": rel_doc,
                    "source_location": f"L1-L{d['lines']}",
                }
            )
            valid_ids.add(doc_id)

        for sec in d.get("sections", [])[:10]:
            sec_id = slugify_id(f"{rel_doc}_{sec['title']}")
            if sec_id not in valid_ids:
                sec_cid = classify_domain_community(rel_doc, sec["title"])
                clean_nodes.append(
                    {
                        "id": sec_id,
                        "label": sec["title"],
                        "_origin": "semantic",
                        "community": sec_cid,
                        "community_name": DOMAIN_COMMUNITIES[sec_cid][0],
                        "file_type": "document",
                        "norm_label": sec["title"].lower(),
                        "source_file": rel_doc,
                        "source_location": f"L{sec['start_line']}-L{sec['end_line']}",
                    }
                )
                valid_ids.add(sec_id)
                clean_links.append(
                    {
                        "source": doc_id,
                        "target": sec_id,
                        "relation": "contains_section",
                        "_origin": "semantic",
                        "confidence": "EXTRACTED",
                        "confidence_score": 1.0,
                        "source_file": rel_doc,
                        "source_location": f"L{sec['start_line']}",
                        "weight": 1.0,
                    }
                )

    # Obtener HEAD actual
    try:
        head_commit = subprocess.run(
            ["git", "rev-parse", "--short", "HEAD"],
            cwd=str(RAIZ),
            capture_output=True,
            text=True,
            encoding="utf-8",
            timeout=5,
        ).stdout.strip() or "HEAD"
    except Exception:
        head_commit = "HEAD"

    g_data["nodes"] = clean_nodes
    g_data["links"] = clean_links
    g_data["built_at_commit"] = head_commit
    graph_path.write_text(json.dumps(g_data, ensure_ascii=False, indent=2), encoding="utf-8")

    # 4. Limpiar manifest.json y agregar los documentos indexados
    if manifest_path.exists():
        try:
            manifest = json.loads(manifest_path.read_text(encoding="utf-8"))
        except Exception:
            manifest = {}
    else:
        manifest = {}

    manifest = {k: v for k, v in manifest.items() if not is_excluded(k)}
    now_ts = time.time()
    for d in all_docs:
        manifest[d["path"]] = {"mtime": now_ts, "type": "document"}
    for w_path in wgsl_map:
        manifest[w_path] = {"mtime": now_ts, "type": "code"}
    manifest_path.write_text(json.dumps(manifest, ensure_ascii=False, indent=2), encoding="utf-8")

    # 5. Generar Wiki navegable para agentes (graphify-out/wiki/index.md + páginas por comunidad)
    comm_nodes: dict[int, list[dict]] = {cid: [] for cid in DOMAIN_COMMUNITIES}
    for n in clean_nodes:
        c = n.get("community", 6)
        if c not in comm_nodes:
            c = 6
        comm_nodes[c].append(n)

    wiki_lines = [
        "# 🧠 Wiki de Conocimiento y Fuentes de Verdad — Blazing Recomp",
        "",
        f"> **Commit base:** `{head_commit}` | **Nodos activos:** `{len(clean_nodes)}` (`0` de `docs/historico/`) | **Enlaces:** `{len(clean_links)}`",
        "> Este índice conecta las **Fuentes de Verdad del Hardware GBA / Android TBDR / AAudio**, los **ADRs** y el **Código Fuente en Rust, C11 y WGSL**.",
        "",
        "## Comunidades de Dominio (Navegación Rápida)",
        "",
        "| ID | Comunidad de Dominio | Nodos | Artículo Detallado | Descripción |",
        "| :---: | :--- | :---: | :--- | :--- |",
    ]

    for cid, (cname, cdesc) in DOMAIN_COMMUNITIES.items():
        nodes_in_c = comm_nodes.get(cid, [])
        art_fname = f"comunidad_{cid}.md"
        wiki_lines.append(
            f"| **{cid}** | **{cname}** | `{len(nodes_in_c)}` | [`{art_fname}`]({art_fname}) | {cdesc} |"
        )

        # Generar artículo de cada comunidad
        doc_items = [n for n in nodes_in_c if n.get("file_type") == "document"]
        code_items = [n for n in nodes_in_c if n.get("file_type") != "document"]
        art_lines = [
            f"# Comunidad {cid}: {cname}",
            "",
            f"> {cdesc}",
            "",
            "## 1. Fuentes de Verdad y Documentación en este Dominio",
            "",
        ]
        if doc_items:
            for dn in doc_items[:40]:
                art_lines.append(
                    f"- **{dn['label']}** (`{dn.get('source_file')}:{dn.get('source_location', 'L1')}`)"
                )
        else:
            art_lines.append("- *(Sin documentos específicos en esta comunidad)*")

        art_lines.extend(
            [
                "",
                "## 2. Símbolos y Módulos de Código Principal (`src/` y `.wgsl`)",
                "",
            ]
        )
        for cn in code_items[:60]:
            if cn.get("source_file"):
                art_lines.append(
                    f"- `{cn['label']}` — `{cn['source_file']}:{cn.get('source_location', 'L1')}`"
                )

        (wiki_dir / art_fname).write_text("\n".join(art_lines) + "\n", encoding="utf-8")

    wiki_lines.extend(
        [
            "",
            "## Catálogo Directo de Fuentes de Verdad (`docs/fuentes/` y `docs/referencia/`)",
            "",
        ]
    )
    for d in fuentes_docs + ref_docs:
        secs_str = " | ".join(f"{s['title']} (`L{s['start_line']}-L{s['end_line']}`)" for s in d["sections"][:6])
        wiki_lines.append(f"- **[`{d['path']}`](../../{d['path']})** ({d['lines']}L): {secs_str}")

    wiki_lines.extend(
        [
            "",
            "## Mapa de Símbolos en Shaders GPU (`src/*.wgsl`)",
            "",
        ]
    )
    for rel_w, winfo in wgsl_map.items():
        syms_str = ", ".join(
            f"`{s['name']}` (`L{s['start_line']}-L{s['end_line']}`)" for s in winfo["symbols"]
        )
        wiki_lines.append(f"- **[`{rel_w}`](../../{rel_w})** ({winfo['lines']}L): {syms_str}")

    (wiki_dir / "index.md").write_text("\n".join(wiki_lines) + "\n", encoding="utf-8")

    # 6. Actualizar GRAPH_REPORT.md sin referencias a docs/historico/ y con comunidades etiquetadas
    deg = Counter()
    for e in clean_links:
        deg[e["source"]] += 1
        deg[e["target"]] += 1
    id_to_node = {n["id"]: n for n in clean_nodes}
    top_gods = []
    for nid, count in deg.most_common(12):
        node_obj = id_to_node.get(nid, {})
        lbl = node_obj.get("label", nid)
        sf = node_obj.get("source_file", "")
        top_gods.append(f"1. `{lbl}` ({sf}) - {count} edges")

    rep_lines = [
        f"# Graph Report - blazing-recomp ({datetime.now().strftime('%Y-%m-%d')})",
        "",
        "## Summary",
        f"- {len(clean_nodes)} nodes · {len(clean_links)} edges · {len(DOMAIN_COMMUNITIES)} domain communities",
        f"- Built from commit: `{head_commit}` (Excluye `docs/historico/`)",
        "- Wiki navegable para agentes: [`graphify-out/wiki/index.md`](wiki/index.md)",
        "",
        "## Domain Communities",
    ]
    for cid, (cname, cdesc) in DOMAIN_COMMUNITIES.items():
        rep_lines.append(f"- **Comunidad {cid} — {cname}** ({len(comm_nodes.get(cid, []))} nodos): {cdesc}")

    rep_lines.extend(["", "## God Nodes (Core Abstractions & Truth Sources)"])
    rep_lines.extend(top_gods)
    report_path.write_text("\n".join(rep_lines) + "\n", encoding="utf-8")

    return {"nodes": len(clean_nodes), "edges": len(clean_links)}


def build_sources_index() -> dict:
    TELEMETRY_DIR.mkdir(parents=True, exist_ok=True)

    fuentes_dir = RAIZ / "docs" / "fuentes"
    ref_dir = RAIZ / "docs" / "referencia"
    exp_dir = RAIZ / "docs" / "explicacion"

    fuentes_docs = [
        parse_markdown_sections(p)
        for p in sorted(fuentes_dir.glob("*.md"))
        if p.name.lower() != "readme.md"
    ] if fuentes_dir.exists() else []

    ref_docs = [
        parse_markdown_sections(p)
        for p in sorted(ref_dir.glob("*.md"))
        if p.name.lower() != "readme.md"
    ] if ref_dir.exists() else []

    glossary_p = RAIZ / "GLOSSARY.md"
    if glossary_p.exists():
        ref_docs.append(parse_markdown_sections(glossary_p))

    exp_docs = [
        parse_markdown_sections(p)
        for p in sorted(exp_dir.glob("*.md"))
        if p.name.lower() != "readme.md"
    ] if exp_dir.exists() else []

    adrs = extract_adrs_from_decisiones()
    wgsl_map = parse_wgsl_shaders()
    rust_mods = scan_rust_src_thermometer()
    purged_cg = clean_codegraph_db()
    graph_stats = update_graphify_and_wiki(fuentes_docs, ref_docs, exp_docs, wgsl_map)

    index_payload = {
        "updated_at": datetime.now().strftime("%Y-%m-%d %H:%M:%S"),
        "max_doc_mtime": max(
            (
                p.stat().st_mtime
                for folder in (fuentes_dir, ref_dir, exp_dir, RAIZ / "src")
                if folder.exists()
                for p in folder.glob("*")
                if p.is_file()
            ),
            default=0.0,
        ),
        "fuentes": fuentes_docs,
        "referencia": ref_docs,
        "explicacion": exp_docs,
        "adrs": adrs,
        "wgsl": wgsl_map,
        "rust_thermometer": rust_mods,
        "graphify": graph_stats,
        "codegraph_purged_historico": purged_cg,
    }

    SOURCES_INDEX_PATH.write_text(
        json.dumps(index_payload, ensure_ascii=False, indent=2),
        encoding="utf-8",
    )
    return index_payload


def main() -> int:
    try:
        sys.stdout.reconfigure(encoding="utf-8", errors="replace")
    except Exception:
        pass
    idx = build_sources_index()
    print(
        json.dumps(
            {
                "status": "ok",
                "fuentes_indexadas": len(idx["fuentes"]),
                "referencia_indexadas": len(idx["referencia"]),
                "explicacion_indexadas": len(idx["explicacion"]),
                "wgsl_indexados": len(idx["wgsl"]),
                "graphify_nodes": idx["graphify"]["nodes"],
                "codegraph_purged": idx["codegraph_purged_historico"],
            },
            ensure_ascii=False,
        )
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
