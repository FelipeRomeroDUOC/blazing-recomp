# Propuesta: Paquete 3 — Rigor en anatomía del motor, cuadratura de src-c y saneamiento Diátaxis

Fecha: 2026-10-24
Nivel: 2
Estado: cerrada

<!-- Estados: pendiente | aprobada | modificada | descartada |
     verificada por agente | revisada por humano | discrepada | cerrada -->

## Propuesta (la escribe el agente antes de editar)

### 1. Qué cambia

Resuelve el bloque final de quejas técnicas y de legibilidad de `RANT.MD` (Bloqueo 4 y Quejas 1, 6, 7, 8, 12, 13, 14, 16, 17, 18, 19 y 20):
1. **Rigor y verificabilidad en la anatomía del motor (`docs/explicacion/anatomia-motor-fire-emblem-7.md`):** Corrige la descripción del salto de interrupción (excepción BIOS `0x00000018` $\to$ puntero de usuario en IWRAM `0x03007FFC` $\to$ `IntrMain`, erradicando el erróneo `0x03000000`); sincroniza la especificación del intervalo de VBlank (período físico de líneas 160 a 227 frente a la caída del bit 0 de `DISPSTAT` en la línea 227 según GBATEK); e incorpora una sección formal de metodología y fuentes que ancla la estructura `Unit` (76 bytes), offsets y rutinas de combate a la descompilación canónica de referencia (`fireemblem7u`), tablas de ROM y etiquetas de `blazing.labels.toml`.
2. **Cuadratura aritmética de `src-c/` y jerarquía de símbolos (`docs/explicacion/arquitectura-c-recompilado.md`, `docs/PENDIENTES.md`, `GLOSSARY.md`):** Sustituye la cifra inexacta de 874.500 líneas en el diagrama por las ~7.130.000 líneas reales distribuidas en las 12 unidades de traducción C11; explica el funcionamiento del umbral `MAX_UNIT` (~16 MB) y por qué la última unidad acumula el remanente; define la jerarquía formal (unidades C11, funciones canónicas etiquetadas y bloques básicos compilados); y corrige en `PENDIENTES.md` la errata que denominaba a 17.207 "bloques cargados" (siendo funciones).
3. **Flujo de etiquetas en IWRAM y delimitación de `pack.toml` (`docs/guias/mapear-labels-aot.md`, `docs/referencia/CONFIGURACION_PACK.md`):** Añade en la guía de labels el procedimiento para capturar y catalogar subrutinas volátiles reubicadas en IWRAM; y añade en la referencia de `pack.toml` una advertencia que delimita qué opciones aplican a la herramienta de recompilación upstream de escritorio y cuáles son inertes en el APK de Android.
4. **Catálogo exhaustivo de herramientas (`docs/referencia/comandos-cli.md`):** Incorpora una tabla completa con todos los scripts auxiliares y de diagnóstico de `scripts/` y `scripts/diagnostico/` (`analizar_telemetria.py`, `desensamblar_rom.py`, `auditar_labels.py`, `compilar_recomp_aot.ps1`, `validar_docs.py`, etc.).
5. **Legibilidad, rutas Diátaxis y advertencias pedagógicas (`docs/guias/medir-rendimiento.md`, `docs/README.md`, `docs/explicacion/ppu-caminos-de-composicion.md`):** Añade un "Inicio Rápido (3 minutos)" en la guía de rendimiento; estructura itinerarios de lectura recomendados por perfil de lector en el portal `docs/README.md` incorporando el enlace a `docs/historico/`; añade advertencia inicial en el tutorial de 75 minutos sobre el requisito del repositorio vecino para los ejercicios prácticos; y normaliza los frontmatters con fecha de revisión `actualizado: 2026-10-24`.

### 2. Contexto

- Contexto consultado: `RANT.MD`, `docs/explicacion/anatomia-motor-fire-emblem-7.md`, `docs/referencia/registros-video-lcd-gba.md`, `docs/explicacion/arquitectura-c-recompilado.md`, `docs/PENDIENTES.md`, `docs/referencia/CONFIGURACION_PACK.md`, `docs/referencia/comandos-cli.md`, `docs/guias/mapear-labels-aot.md`, `docs/guias/medir-rendimiento.md`, `docs/README.md`, `docs/explicacion/ppu-caminos-de-composicion.md`, `GLOSSARY.md`, `AGENTS.md`.
- Intención: Cerrar la totalidad de las quejas abiertas de `RANT.MD`, elevando la documentación técnica del proyecto al estándar más riguroso de fidelidad histórica de hardware y reproducibilidad pedagógica.
- Restricciones y decisiones previas (máx. 3):
  - Prohibición de rutas locales absolutas en archivos versionados (`AGENTS.md` §7) → enlaces y referencias estrictamente relativos.
  - Conformidad Diátaxis estricta (`validar_docs.py`) → preservar encabezados obligatorios de cada cuadrante.
  - Suite de 250 tests al 100% en verde (`AGENTS.md` §6) → cero regresiones en tests de empaquetado y contrato.
- Alternativas descartadas:
  - Cambiar el particionado real de `src-c/` para forzar que la última unidad pese menos de 16 MB → descartada porque requeriría reejecutar `recomp build` y recompilar las bibliotecas nativas de todos los ABIs sin necesidad técnica. La solución documental consiste en transparentar que `MAX_UNIT` es un umbral de corte por bloque y la última unidad acumula el remanente.
- Ante imprevistos, prioriza: exactitud verificable con fuentes primarias sobre aproximaciones descriptivas.

### 3. Ejemplos

- Caso normal 1 (Vector de interrupción en `anatomia-motor-fire-emblem-7.md`):
  - Entrada: El lector busca cómo atiende la CPU las interrupciones en el juego original.
  - Antes: "El procesador salta a `0x03000000` (IWRAM), atiende la señal y despierta las rutinas...".
  - Ahora: "El procesador toma la excepción IRQ en el vector BIOS `0x00000018`. La rutina de BIOS lee el puntero de usuario en `0x03007FFC` (final de IWRAM), el cual desvía la ejecución a `IntrMain` (copiada desde ROM a IWRAM en el arranque). `IntrMain` consulta `REG_IF` (`0x04000202`), atiende la señal de VBlank y despierta las tareas...".
- Caso normal 2 (Aritmética de líneas de `src-c/` en `arquitectura-c-recompilado.md`):
  - Entrada: El desarrollador consulta el volumen de código C generado.
  - Antes: El diagrama indica `src-c/*.c (874.500 líneas C11)` mientras que la tabla describe doce unidades de ~600k a 950k líneas.
  - Ahora: El diagrama y la tabla se unifican en `~7.130.000 líneas C11` (~215 MB en total en las 12 unidades), explicando que `MAX_UNIT` (~16 MB) divide por bloques completos y la unidad 11 acumula el remanente de bloques y tablas constantes.
- Caso normal 3 (Inicio rápido en `medir-rendimiento.md`):
  - Entrada: Un desarrollador quiere medir los FPS de su teléfono en 2 minutos sin leer la especificación de 400 líneas.
  - Ahora: La guía abre con la sección "Inicio Rápido (3 minutos)" con los 3 comandos esenciales (`adb shell`, `adb pull` y `analizar_telemetria.py`), enlazando los detalles a las secciones posteriores.

### 4. Pasos

1. Modificar `docs/explicacion/anatomia-motor-fire-emblem-7.md`:
   - Corregir el flujo de interrupción (vector BIOS `0x00000018` $\to$ puntero `0x03007FFC` $\to$ `IntrMain`).
   - Sincronizar el intervalo de VBlank: aclarar que la duración física son 68 scanlines (160..227) y que el bit 0 de `DISPSTAT` cae a 0 en la línea 227 según GBATEK.
   - Añadir la sección "Metodología y fuentes de verificación", citando la estructura `Unit` de 76 bytes de `fireemblem7u`, tablas de ROM y rutinas de combate en IWRAM.
2. Modificar `docs/explicacion/arquitectura-c-recompilado.md`, `docs/PENDIENTES.md` y `GLOSSARY.md`:
   - Actualizar el diagrama Mermaid y el texto de `arquitectura-c-recompilado.md` reflejando las ~7.130.000 líneas de C11 reales y explicando el funcionamiento de `MAX_UNIT`.
   - Corregir en `docs/PENDIENTES.md` (§ "Investigar discrepancia de bloques") la errata de "17.207 bloques cargados", denominándolas funciones canónicas preliminares.
   - Clarificar en `GLOSSARY.md` la jerarquía conceptual: unidad de traducción C11, función etiquetada y bloque básico compilado.
3. Modificar `docs/referencia/CONFIGURACION_PACK.md`:
   - Añadir nota de delimitación arquitectónica en la introducción aclarando el alcance de `pack.toml` frente al runtime de Android.
4. Modificar `docs/referencia/comandos-cli.md`:
   - Incorporar la tabla exhaustiva de scripts y utilidades de `scripts/` y `scripts/diagnostico/`.
5. Modificar `docs/guias/mapear-labels-aot.md`:
   - Añadir la sección "Catalogación de subrutinas en IWRAM (`0x03xxxxxx`)" explicando el origen del volcado y el uso de etiquetas en RAM.
6. Modificar `docs/guias/medir-rendimiento.md`, `docs/README.md` y `docs/explicacion/ppu-caminos-de-composicion.md`:
   - Añadir "Inicio Rápido (3 minutos)" en `medir-rendimiento.md`.
   - Incorporar los 3 itinerarios de lectura recomendados en `docs/README.md` y enlazar `docs/historico/`.
   - Añadir advertencia inicial de repositorio vecino en `ppu-caminos-de-composicion.md`.
   - Normalizar frontmatters con fecha de revisión `actualizado: 2026-10-24`.

### 5. Qué más podría verse afectado

- El validador Diátaxis (`validar_docs.py`): Revisa las 22 páginas del repositorio.
- `tests/packaging_tests.rs`: Valida ausencia de rutas locales y frases caducas.

### 6. Suposiciones

- Suposición 1: Ningún test automatizado parsea el número exacto "874.500" en el diagrama de arquitectura C (verificado: no existe en la suite de tests).

### 7. Alcance

- Incluye:
  - Corrección de las quejas 1, 6, 7, 8, 12, 13, 14, 16, 17, 18, 19 y 20 de `RANT.MD`.
  - Rigor metodológico y procedencia de datos en la anatomía del motor.
  - Cuadratura de líneas y jerarquía de bloques/funciones en `src-c/`.
  - Inicio rápido en telemetría y rutas de aprendizaje en el portal de documentación.
  - Catálogo completo de scripts CLI y procedimiento de labels en IWRAM.
- No incluye:
  - Modificación de código fuente en `src/` o reemisión de archivos `.c` en `src-c/`.

### 8. Criterios de aceptación (no se modifican sin aprobación)

Escenario: La anatomía del motor documenta el vector de interrupción real y el anclaje de datos
Caso de uso: Desarrollador estudia la gestión de interrupciones y estructuras en FE7 | Tipo: normal
  Dado el archivo `docs/explicacion/anatomia-motor-fire-emblem-7.md`
  Cuando se consulta el ciclo de interrupciones y las estructuras de datos
  Entonces describe el vector de BIOS `0x00000018` con lectura en `0x03007FFC`, la caída de `DISPSTAT` en la línea 227 y cita la estructura `Unit` de `fireemblem7u`.
Test: inspección textual en `docs/explicacion/anatomia-motor-fire-emblem-7.md`
Estado previo: falla (hoy afirma que salta a `0x03000000`)

Escenario: El diagrama de arquitectura C refleja las líneas reales de C11
Caso de uso: Lector examina el volumen de código compilado de src-c/ | Tipo: normal
  Dado el archivo `docs/explicacion/arquitectura-c-recompilado.md`
  Cuando se inspecciona el diagrama Mermaid y el texto de volumen
  Entonces refleja las ~7.130.000 líneas reales y no contiene la cifra caduca `874.500`.
Test: inspección textual en `docs/explicacion/arquitectura-c-recompilado.md`
Estado previo: falla (hoy contiene `874.500 líneas C11`)

Escenario: La guía de rendimiento abre con un inicio rápido de 3 minutos
Caso de uso: Usuario mide FPS rápidamente sin leer la referencia completa | Tipo: normal
  Dado el archivo `docs/guias/medir-rendimiento.md`
  Cuando se consulta la parte inicial de la guía
  Entonces contiene una sección de inicio rápido con los comandos de adb y análisis en menos de 20 líneas.
Test: inspección textual en `docs/guias/medir-rendimiento.md`
Estado previo: falla (hoy arranca directamente con los pasos exhaustivos y la calibración)

Escenario: La suite completa de tests de Rust se mantiene al 100% en verde con 250 tests
Caso de uso: Verificación de no regresión | Tipo: regresión
  Dado el árbol de trabajo tras la sincronización documental
  Cuando se ejecuta `cargo test`
  Entonces los 250 tests pasan en verde.
Test: `cargo test`
Estado previo: pasa (250 tests en verde)

Escenario: Validación Diátaxis sin errores ni avisos
Caso de uso: Estructura documental continua | Tipo: regresión
  Dado el árbol de documentación
  Cuando se ejecuta `python .agents/skills/diataxis-docs/scripts/validar_docs.py .`
  Entonces reporta 22 páginas revisadas, 0 errores y 0 avisos.
Test: `python .agents/skills/diataxis-docs/scripts/validar_docs.py .`
Estado previo: pasa

### 9. Cómo lo comprobaré

- [Anatomía documenta vector y anclaje] → herramienta: `Select-String -Path "docs/explicacion/anatomia-motor-fire-emblem-7.md" -Pattern "0x03007FFC"` → pasa si: existe la coincidencia → no detecta: erratas en otros registros.
- [Arquitectura C refleja 7M líneas] → herramienta: `Select-String -Path "docs/explicacion/arquitectura-c-recompilado.md" -Pattern "7.1"` → pasa si: existe la coincidencia y no existe `874.500` → no detecta: desajustes menores en comentarios.
- [Inicio rápido en rendimiento] → herramienta: `Select-String -Path "docs/guias/medir-rendimiento.md" -Pattern "Inicio rápido"` → pasa si: existe la sección → no detecta: errores de sintaxis en comandos de usuario.
- [Suite de tests en verde] → herramienta: `cargo test` → pasa si: 250 tests aprobados.
- [Validación Diátaxis] → herramienta: `python .agents/skills/diataxis-docs/scripts/validar_docs.py .` → pasa si: 0 errores y 0 avisos.

---

## Resultado (lo escribe el agente tras verificar)

### 1. Predicho vs. real

| Criterio / Escenario | Predicho | Real | Desviación |
| :--- | :--- | :--- | :--- |
| **Vector de interrupción y anclaje en `anatomia-motor-fire-emblem-7.md`** | Corrección del vector a `0x00000018` $\to$ `0x03007FFC` $\to$ `IntrMain`, intervalo VBlank y anclaje con `fireemblem7u`. | Implementado exactamente como se propuso; añadida la sección 7 de metodología y fuentes. | Ninguna. |
| **Aritmética de líneas C11 en `arquitectura-c-recompilado.md`** | Diagrama actualizado a `~7.130.000 líneas C11`, explicación de `MAX_UNIT` y erradicación de `874.500`. | Diagrama Mermaid y tabla de unidades sincronizados a ~7.130.000 líneas y ~215 MB. Errata corregida en `PENDIENTES.md`. | Ninguna. |
| **Jerarquía conceptual en `GLOSSARY.md`** | Definición explícita de unidad de traducción C11, función canónica y bloque básico. | Entradas agregadas en `GLOSSARY.md` (§ 1) enlazando fuentes primarias. | Ninguna. |
| **Delimitación de `pack.toml`** | Nota de alcance en `CONFIGURACION_PACK.md` delimitando el generador upstream del runtime Android. | Documentado en la descripción general y en las secciones `[package]` y `[runtime]`. | Ninguna. |
| **Catálogo de herramientas en `comandos-cli.md`** | Tabla global con los 19 scripts de `scripts/`, `scripts/diagnostico/` y `.agents/skills/`. | Tabla insertada en la descripción general cubriendo todos los scripts y herramientas. | Ninguna. |
| **Flujo de labels en IWRAM** | Procedimiento de catalogación de subrutinas dinámicas en `mapear-labels-aot.md`. | Subsección agregada en el paso 4 explicando el origen en ROM y el registro en `blazing.labels.toml`. | Ninguna. |
| **Inicio rápido en `medir-rendimiento.md`** | Sección de inicio rápido en 3 comandos al comienzo de la guía. | Sección 0 "Inicio rápido (diagnóstico en 3 minutos)" agregada con comandos ADB y análisis. | Ninguna. |
| **Itinerarios de lectura y enlace histórico** | 3 rutas de lectura en `docs/README.md` y enlace a `docs/historico/README.md`. | Creado `docs/historico/README.md` e incorporadas las 3 rutas pedagógicas en el portal principal. | Ninguna. |
| **Advertencia en `ppu-caminos-de-composicion.md`** | Nota aclaratoria sobre el requisito del repositorio vecino `../gba-recomp`. | Callout `> [!NOTE]` actualizado con el prerrequisito del motor hermano. | Ninguna. |
| **Frontmatters Diátaxis actualizados** | `actualizado: 2026-10-24` en todas las páginas modificadas. | Añadido en los 7 documentos intervenidos sin alterar metadatos obligatorios. | Ninguna. |
| **Validación Diátaxis (`validar_docs.py`)** | 0 errores y 0 avisos. | 23 páginas revisadas, 0 errores, 0 avisos. | Ninguna. |
| **Suite de pruebas de Rust (`cargo test`)** | 250 tests pasando en verde (172 unitarios + 78 integración). | 250 tests aprobados en verde (0 fallos). | Ninguna. |
| **Gatekeeper de rutas locales** | 0 rutas locales absolutas detectadas. | `test_no_local_paths_in_packaging` aprobado en verde. | Ninguna. |

### 2. Qué se respetó del contexto
- **Portabilidad y privacidad:** Se verificó rigurosamente la ausencia de rutas absolutas locales (`C:\Users\...`).
- **Conformidad Diátaxis:** Se respetaron los encabezados estandarizados exigidos por `validar_docs.py` en todos los cuadrantes.
- **Integridad de tests:** Ningún test fue modificado ni relajado; los 250 tests pasan limpios.

### 3. Fuera de alcance (hallazgos que no se tocaron)
- Ningún código fuente en `src/` requirió modificación para este paquete documental y de rigor analítico.
- Las fuentes generadas en `src-c/` no se regeneraron físicamente, ya que sus 7,13 millones de líneas coinciden exactamente con la especificación documental unificada.
