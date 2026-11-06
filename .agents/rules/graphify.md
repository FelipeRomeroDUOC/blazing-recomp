## Protocolo de Enrutamiento Dual: CodeGraph vs. Graphify

Este proyecto utiliza **dos grafos complementarios** libres de código histórico retirado (`docs/historico/` está excluido de ambos):

1. **CodeGraph (`.codegraph/`) — Para Código Fuente Vivo y Call-Graph:**
   - Úsalo primero cuando necesites leer funciones verbatim en Rust (`src/`, `tests/`), Python (`scripts/`) o C, ver quién llama a un símbolo (*blast radius*) o qué tests unitarios lo cubren:
     - MCP: `call_mcp_tool(ServerName="codegraph", ToolName="codegraph_explore", Arguments={"query": "<símbolos>", "projectPath": "<ruta_repo>"})`
     - CLI: `codegraph.cmd explore "<símbolos>"`
2. **Graphify (`graphify-out/`) — Para Fuentes de Verdad (GBA / Android TBDR / AAudio / FE7), ADRs y Shaders WGSL:**
   - Úsalo cuando la consulta involucre cómo funciona el hardware de Game Boy Advance, las GPUs móviles TBDR, el motor de *Fire Emblem 7*, los ADRs en `docs/explicacion/DECISIONES.md` o la ubicación de funciones en `src/*.wgsl`:
     - Navega primero [`graphify-out/wiki/index.md`](../../graphify-out/wiki/index.md) (y sus artículos `comunidad_0.md` a `comunidad_6.md`) o consulta el índice precalculado en `.agents/telemetry/sources_index.json`.
     - Lee únicamente el rango de líneas relevante (`StartLine` y `EndLine`) indicado en el índice de secciones.
   - Tras modificar archivos en `src/`, `docs/fuentes/`, `docs/referencia/` o `docs/explicacion/`, el hook `post_tool_runner.py` ejecuta automáticamente `python scripts/indexar_contexto_y_fuentes.py` para mantener sincronizados `graphify-out/graph.json`, `graphify-out/wiki/index.md` y `.codegraph/codegraph.db`.
