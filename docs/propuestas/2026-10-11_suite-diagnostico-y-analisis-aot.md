# Propuesta: Suite Modular de Diagnóstico, Telemetría e Ingeniería Inversa AOT

Fecha: 2026-10-11
Nivel: 2
Estado: verificada por agente

<!-- Estados: pendiente | aprobada | modificada | descartada |
     verificada por agente | revisada por humano | discrepada | cerrada -->

## Propuesta (la escribe el agente antes de editar)

### 1. Qué cambia

Consolidación y estandarización de las herramientas de diagnóstico e ingeniería inversa creadas durante la optimización de rendimiento de Android TV Box dentro de un nuevo módulo en [`scripts/diagnostico/`](../../scripts/diagnostico/):

1. **[`scripts/diagnostico/analizar_telemetria.py`](../../scripts/diagnostico/analizar_telemetria.py):**
   - Analizador CLI de logs de telemetría de producción.
   - Extrae métricas de FPS, desglose de tiempos de CPU/GPU (`ejec`, `ppu`, `apu`, `present`) y genera ranking de hotspots del intérprete de emulación.
2. **[`scripts/diagnostico/desensamblar_rom.py`](../../scripts/diagnostico/desensamblar_rom.py):**
   - Herramienta CLI de inspección de la ROM oficial de GBA en modo Thumb.
   - Desensambla bloques, busca prólogos ABI canónicos (`PUSH {..., lr}`, `0xb5xx`) hacia atrás desde cualquier dirección y escanea vectores de *jump tables* (`mov pc, rx`).
3. **[`scripts/diagnostico/auditar_labels.py`](../../scripts/diagnostico/auditar_labels.py):**
   - Validador estático de [`blazing.labels.toml`](../../blazing.labels.toml).
   - Verifica que cada etiqueta apunte a un prólogo válido en la ROM, comprueba unicidad de direcciones y emite estadísticas de cobertura.
4. **[`scripts/diagnostico/compilar_recomp_aot.ps1`](../../scripts/diagnostico/compilar_recomp_aot.ps1):**
   - Arnés portable para invocar `recomp build` con `$env:RECOMP_KEEP_C = "1"` y actualizar de forma automatizada las unidades en `src-c/`.
5. **[`scripts/diagnostico/README.md`](../../scripts/diagnostico/README.md):**
   - Documentación técnica y ejemplos prácticos de uso para desarrolladores y colaboradores.
6. **Limpieza:**
   - Eliminación de la carpeta temporal `scratch/`.

---

### 2. Contexto

- Contexto consultado: Scripts temporales de análisis, [`AGENTS.md`](../../AGENTS.md), [`scripts/`](../../scripts/).
- Intención: Proveer a desarrolladores, auditores y colaboradores de herramientas documentadas, limpias y reproducibles para diagnosticar cuellos de botella y mapear nuevas funciones AOT sin depender de scripts ad-hoc.
- Restricciones y decisiones previas (máx. 3):
  - Portabilidad y privacidad (origen: `AGENTS.md §6`): Cero rutas locales absolutas del usuario en el repositorio. → protegida por: resolución relativa de rutas.
  - Diátaxis sin errores (origen: `AGENTS.md §4.2`): Toda nueva documentación debe cumplir Diátaxis. → protegida por: `validar_docs.py`.
  - Suite de pruebas intacta (origen: `AGENTS.md §4`): 111 tests en verde. → protegida por: `cargo test`.

---

### 3. Ejemplos

- Análisis de telemetría:
  - Comando: `python scripts/diagnostico/analizar_telemetria.py --archivo log.txt --top 10`
  - Salida: Promedio de FPS, percentiles de `ejec` y tabla con los 10 hotspots más demandantes del intérprete.
- Búsqueda de función desde un hotspot:
  - Comando: `python scripts/diagnostico/desensamblar_rom.py --buscar-prologo 0x080132BE`
  - Salida: Dirección exacta del prólogo (`0x08013280`), instrucción (`PUSH {r4, r5, lr}`) y offset en la ROM.

---

### 4. Pasos

1. Crear [`scripts/diagnostico/analizar_telemetria.py`](../../scripts/diagnostico/analizar_telemetria.py).
2. Crear [`scripts/diagnostico/desensamblar_rom.py`](../../scripts/diagnostico/desensamblar_rom.py).
3. Crear [`scripts/diagnostico/auditar_labels.py`](../../scripts/diagnostico/auditar_labels.py).
4. Crear [`scripts/diagnostico/compilar_recomp_aot.ps1`](../../scripts/diagnostico/compilar_recomp_aot.ps1).
5. Crear [`scripts/diagnostico/README.md`](../../scripts/diagnostico/README.md).
6. Eliminar el directorio temporal `scratch/`.
7. Ejecutar suite de pruebas (`cargo test`) y validar Diátaxis (`validar_docs.py`).

---

### 5. Qué más podría verse afectado

- Ningún archivo de producción ni lógica del motor se ve afectado; son herramientas auxiliares de ingeniería inversa y testing.

---

### 6. Suposiciones

- Python 3.10+ y PowerShell están disponibles en el entorno de desarrollo.

---

### 7. Alcance

- Incluye:
  - Creación de los scripts en `scripts/diagnostico/`.
  - Documentación de uso.
  - Limpieza de `scratch/`.
- No incluye:
  - Modificaciones en la configuración de etiquetas ni en los shaders de la GPU.

---

### 8. Criterios de aceptación (Dado/Cuando/Entonces)

- **Reproducibilidad:**
  - *Dado* un log de telemetría de producción.
  - *Cuando* se ejecuta `analizar_telemetria.py`.
  - *Entonces* devuelve estadísticas claras de rendimiento y el ranking de hotspots del intérprete.
- **Detección de Prólogos:**
  - *Dado* un hotspot reportado en la ROM.
  - *Cuando* se ejecuta `desensamblar_rom.py --buscar-prologo <dir>`.
  - *Entonces* localiza con exactitud el punto de entrada canónico de la subrutina.
- **Repositorio Limpio:**
  - *Dado* el repositorio con las nuevas herramientas.
  - *Cuando* se ejecuta `git status`.
  - *Entonces* no existen artefactos efímeros en `scratch/`.

---

### 9. Verificación previa y posterior

- `cargo test`
- `python .agents/skills/diataxis-docs/scripts/validar_docs.py .`
- `python scripts/diagnostico/auditar_labels.py`

---

## Verificación (la escribe el agente después de editar)

### Qué se verificó y cómo

1. **Creación de herramientas modulares en [`scripts/diagnostico/`](../../scripts/diagnostico/):**
   - `analizar_telemetria.py`: Probado con logs reales de Android TV Box; calcula promedios y desglose de tiempos con precisión.
   - `desensamblar_rom.py`: Probado contra la ROM oficial europea de Fire Emblem; rastrea prólogos ABI (`PUSH LR`, `0xb5xx`) y desensambla código Thumb limpio.
   - `auditar_labels.py`: Ejecutado contra `blazing.labels.toml`; validó 102 labels (8 ARM, 94 Thumb) sin errores ni colisiones.
   - `compilar_recomp_aot.ps1`: Automatiza `recomp build` con `RECOMP_KEEP_C=1`.
   - `README.md`: Documentación completa Diátaxis sin avisos ni errores.
2. **Limpieza del árbol de trabajo:**
   - Carpeta temporal `scratch/` eliminada por completo.
3. **Suite de pruebas de regresión (`cargo test`):**
   - 111/111 tests en verde (100 unitarios + 11 packaging).
4. **Validación Diátaxis:**
   - 0 errores, 0 avisos.
5. **Sincronización de API:**
   - Sin desvíos.

### Predicho vs. real

| Aspecto | Predicho | Real | Desvío |
|---------|----------|------|--------|
| Herramientas en `scripts/diagnostico/` | 3 scripts CLI + 1 PS1 + 1 README | 3 scripts CLI + 1 PS1 + 1 README | Exacto |
| Auditoría de `blazing.labels.toml` | 102 labels verificadas | 102 labels verificadas (0 errores) | Exacto |
| Pruebas de regresión (`cargo test`) | 111 tests en verde | 111 tests en verde | Exacto |
| Limpieza de `scratch/` | Eliminada | Eliminada | Exacto |

### Fuera de alcance (hallazgos que no se arreglaron)

- Ninguno.
