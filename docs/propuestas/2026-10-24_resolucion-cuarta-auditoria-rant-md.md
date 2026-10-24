# Propuesta: Resolución de Cuarta Auditoría RANT.MD (Saneamiento de BACKLOG, IWRAM, Empaquetado y Legibilidad)

Fecha: 2026-10-24
Nivel: 2
Estado: cerrada

<!-- Estados: pendiente | aprobada | modificada | descartada |
     verificada por agente | revisada por humano | discrepada | cerrada -->

## Propuesta (la escribe el agente antes de editar)

### 1. Qué cambia

Resuelve los 5 bloqueos y las quejas de comprensión y legibilidad identificadas en la cuarta lectura de `RANT.MD`:

1. **Saneamiento integral de `docs/BACKLOG.md`:**
   - **Retiro real del shim y fallback por software:** Actualiza «Áreas de Trabajo y Estado Real» §3 y «Prioridad: lo alto que sigue vivo» eliminando cualquier afirmación obsoleta sobre la presencia de `PpuShim` en el árbol activo. Deja asentado con total claridad que el shim y sus arneses fueron archivados conforme al ADR 42, y que el mecanismo de contingencia/diagnóstico ante fallo de GPU o activación de `ppu-nativa-off.flag` es el rasterizador por software en CPU provisto por `gba-core`.
   - **Sincronización de fecha y recuento de tests:** Actualiza la fecha a 2026-10-24 y el recuento al valor exacto comprobado con `cargo test -- --list`: **257 tests en verde** (172 unitarios de `src/lib.rs` y 85 de integración en `tests/`).
   - **Delimitación del «Registro de Hallazgos» frente a `PENDIENTES.md`:** Clarifica en la cabecera que el registro es histórico y acumulativo (no una lista de tareas pendientes), y remite a [`docs/PENDIENTES.md`](PENDIENTES.md) para las tareas de desarrollo abiertas.
   - **Corrección de enlace a `AGENTS.md`:** Corrige las referencias que apuntaban erróneamente a `AGENTS.md §6` enlazando directamente con la sección §7 (`#checklist-obligatorio-de-pre-commit-gatekeeper`).

2. **Clarificación y procedimiento operativo de IWRAM:**
   - **Unificación de rangos en conflicto:** En `docs/guias/mapear-labels-aot.md` paso 4, desambigua los rangos explicando que `0x030035C8`–`0x030036CC` es el clúster de subrutinas de combate/cálculo de daño en IWRAM que la telemetría reporta como hotspots, mientras que `0x030028A0` es la dirección de destino a la que se copia `IntrMain`.
   - **Diferenciación entre vector y función:** En `docs/guias/mapear-labels-aot.md` paso 4, precisa que `0x03007FFC` es la dirección de la palabra de memoria que aloja el **vector de usuario** (`User IRQ Handler`) que la BIOS consulta para saltar a `IntrMain`, mientras que el código ejecutable de la subrutina reside en `0x030028A0`.
   - **Procedimiento técnico del blob de 32 KB:** En `docs/guias/mapear-labels-aot.md` paso 4 y `docs/PENDIENTES.md`, describe el arnés determinista de replay (`examples/ppu_barrido_nativa.rs --replay`), el volcado del rango `0x03000000`–`0x03007FFF` a `iwram_dump.bin` y su integración en `recomp build` para generar los bloques protegidos con `a->guard`.

3. **Precisión de la PPU por software en `docs/explicacion/arquitectura-port-android.md`:**
   - Corrige la afirmación «Se eliminó la rasterización por software» en Alternativas y Compromisos #2, alineándola fielmente con el ADR 42: la ruta de producción no utiliza emulación en CPU y corre 100 % en GPU (`NativePpu`), mientras que la rasterización por software de `gba-core` (`render_scanline`) se preserva exclusivamente como contingencia y diagnóstico.

4. **Sinceramiento de backend y calibración en `docs/guias/medir-rendimiento.md`:**
   - Corrige la indicación de que la app prefiere OpenGL ES: Vulkan es el backend preferente (`PREFERIR_OPENGL_ES = false`), con respaldo automático a OpenGL ES.
   - Clarifica que la calibración de presentación está actualmente activa en `src/platform.rs` (`CALIBRAR_PRESENTACION = true`) debido al experimento en Android TV Stick (ADR 40), y que su desactivación definitiva es una tarea registrada en `PENDIENTES.md`.

5. **Precisión de labels y símbolos en `docs/explicacion/anatomia-motor-fire-emblem-7.md`:**
   - En §7 punto 3, elimina la mención a «firmas de llamada», ratificando que `blazing.labels.toml` define únicamente identificadores, puntos de entrada y modos (Thumb/ARM).
   - En §1 y §7, aclara que `sub_8059E7C` es la nomenclatura canónica del proyecto comunitario de descompilación `fireemblem7u` correspondiente a la subrutina ubicada en la dirección de ROM `0x08059E7C`.

6. **Inclusión de `libblazing_recompiled.so` en empaquetado:**
   - En `docs/guias/compilar-y-empaquetar-apk.md` paso 2 y `docs/referencia/comandos-cli.md`, explicita que `scripts/package_apk.ps1` empaqueta ambas bibliotecas nativas contiguas en `lib/<abi>/`: `libblazing_recomp.so` (runtime Rust) y `libblazing_recompiled.so` (lógica C11 AOT generada previamente con `compilar_c_paralelo.py`).

7. **Actualización de estado en `docs/explicacion/README.md`:**
   - Actualiza la descripción de `hardware-shim-y-eliminacion-de-emulacion.md` en el catálogo de explicación: marco conceptual con Fase 1 (PPU) sustituida por GPU directa según ADR 42, y Fases 2 (Bus) y 3 (Audio) pendientes.

8. **Normalización de subtítulos en `docs/referencia/FORMATO_LABELS.md`:**
   - Elimina los prefijos residuales `3.1` y `3.2` en los subtítulos de campos y atributos para estricta consistencia Diátaxis (`### Cabecera`, `### Entradas de funciones`).

9. **Incorporación de `examples/` y `telemetria/` en `README.md`:**
   - Añade a la sección «Estructura del Proyecto» los directorios [`examples/`](examples/README.md) (arneses deterministas y suites de validación) y [`telemetria/`](telemetria/esquema.txt) (esquema y contrato único de telemetría).

10. **Portal de documentación (`docs/README.md`) y cuadrantes complementarios:**
    - Incorpora en el diagrama y en el texto del portal las secciones de Fuentes de Verdad Externa (`docs/fuentes/`) y Archivo Histórico (`docs/historico/`), delimitando su rol complementario fuera de los cuatro cuadrantes Diátaxis tradicionales.

11. **Corrección del generador `scripts/extraer_referencias.py` y de `api-modulos-rust.md`:**
    - Modifica `scripts/extraer_referencias.py` para extraer la primera oración completa del docstring de módulo (uniendo líneas hasta el primer punto) y escapar cualquier carácter `|` (`\|`), regenerando `docs/referencia/api-modulos-rust.md` para evitar celdas cortadas o rotas en la tabla.

---

### 2. Contexto

- Contexto consultado: `RANT.MD` (cuarta lectura), `docs/BACKLOG.md`, `docs/PENDIENTES.md`, `docs/guias/mapear-labels-aot.md`, `docs/guias/medir-rendimiento.md`, `docs/guias/compilar-y-empaquetar-apk.md`, `docs/explicacion/arquitectura-port-android.md`, `docs/explicacion/anatomia-motor-fire-emblem-7.md`, `docs/explicacion/README.md`, `docs/referencia/FORMATO_LABELS.md`, `docs/referencia/comandos-cli.md`, `README.md`, `docs/README.md`, `scripts/extraer_referencias.py`.
- Intención: Resolver el 100% de los bloqueos y quejas técnicas de la 4ª auditoría, garantizando coherencia absoluta entre el tablero vivo, las guías prácticas y las explicaciones arquitectónicas.
- Restricciones:
  1. Preservar 0 rutas absolutas locales en cualquier archivo.
  2. Mantener la suite de 257 tests al 100 % en verde.
  3. Mantener 0 errores y 0 avisos en `validar_docs.py`.
  4. Mantener sincronizado `python scripts/extraer_referencias.py --check`.
  5. Mantener sincronizado `python scripts/auditar_hallazgos.py`.

---

### 3. Ejemplos (Antes / Ahora)

- **Caso 1: Estado del shim en `BACKLOG.md`:**
  - *Antes:* «si la GPU del dispositivo rechaza un recurso el guardián de wgpu devuelve el juego al shim»; «El shim de la PPU sigue en el repositorio […] borrar src/ppu_shim.*».
  - *Ahora:* «El PpuShim fue retirado y archivado conforme al ADR 42. Ante fallo de inicialización de GPU o activación de ppu-nativa-off.flag, el fallback de contingencia es la rasterización por software de gba-core».
- **Caso 2: Cifras de tests en `BACKLOG.md`:**
  - *Antes:* «250 tests en verde» / «Última actualización: 2026-10-20».
  - *Ahora:* «257 tests en verde (172 en los módulos de src/, 85 de integración en tests/) a 2026-10-24».
- **Caso 3: PPU software en `arquitectura-port-android.md`:**
  - *Antes:* «Cero emulación de PPU en CPU: Se eliminó la rasterización por software línea a línea (render_scanline)...».
  - *Ahora:* «La ruta de producción no utiliza emulación PPU en CPU (100% NativePpu en GPU); render_scanline en CPU se conserva exclusivamente como contingencia y diagnóstico conforme a ADR 42».
- **Caso 4: Backend en `medir-rendimiento.md`:**
  - *Antes:* «Durante el experimento de presentación la aplicación prefiere OpenGL ES...».
  - *Ahora:* «Vulkan es el backend preferente (PREFERIR_OPENGL_ES = false), con respaldo automático a OpenGL ES si el dispositivo no soporta Vulkan».
- **Caso 5: Desglose de IWRAM y vector en `mapear-labels-aot.md`:**
  - *Antes:* «Despacho de interrupciones (IntrMain, 0x03007FFC / 0x030028A0)».
  - *Ahora:* «Vector de interrupción de usuario en 0x03007FFC (puntero consultado por BIOS), el cual salta a la función ejecutable IntrMain copiada en 0x030028A0».

---

### 4. Pasos de Ejecución

1. Modificar `docs/BACKLOG.md` actualizando el estado de la PPU, la fecha (2026-10-24), el conteo exacto de 257 tests (172 + 85), la naturaleza del registro de hallazgos y el enlace a `AGENTS.md §7`.
2. Modificar `docs/guias/mapear-labels-aot.md` y `docs/PENDIENTES.md` desambiguando los rangos de IWRAM (`0x030035C8`–`0x030036CC` para combate, `0x030028A0` para `IntrMain`, `0x03007FFC` para el vector) y detallando el procedimiento operativo de captura del blob de 32 KB.
3. Modificar `docs/explicacion/arquitectura-port-android.md` (§ Alternativas #2) precisando el rol de contingencia del rasterizador software según ADR 42.
4. Modificar `docs/guias/medir-rendimiento.md` corrigiendo el backend preferente (Vulkan) y el estado activo de `CALIBRAR_PRESENTACION`.
5. Modificar `docs/explicacion/anatomia-motor-fire-emblem-7.md` clarificando `sub_8059E7C` (descompilación `fireemblem7u`) y eliminando la promesa de firmas en las etiquetas.
6. Modificar `docs/guias/compilar-y-empaquetar-apk.md` y `docs/referencia/comandos-cli.md` explicitando el empaquetado de `libblazing_recompiled.so` junto a `libblazing_recomp.so`.
7. Modificar `docs/explicacion/README.md` actualizando la descripción del documento de hardware shim.
8. Modificar `docs/referencia/FORMATO_LABELS.md` normalizando los subtítulos sin números (`### Cabecera`, `### Entradas de funciones`).
9. Modificar `README.md` añadiendo `examples/` y `telemetria/` a la estructura del proyecto.
10. Modificar `docs/README.md` incorporando `docs/fuentes/` y `docs/historico/` en el portal.
11. Modificar `scripts/extraer_referencias.py` para unir líneas de docstrings de módulo hasta el primer punto, escapar pipes y regenerar `docs/referencia/api-modulos-rust.md`.
12. Ejecutar la suite completa de validación (`cargo test`, `validar_docs.py`, `extraer_referencias.py --check`, `auditar_labels.py`, `auditar_hallazgos.py`, `test_no_local_paths_in_packaging`).
13. Cerrar la propuesta y presentar reporte.

---

### 5. Criterios de Aceptación (Dado / Cuando / Entonces)

- **Escenario 1: El BACKLOG declara el retiro del shim y 257 tests**
  - *Dado:* `docs/BACKLOG.md`
  - *Cuando:* Se consultan la cabecera, «Estado del Proyecto» y «Áreas de Trabajo»
  - *Entonces:* El shim figura como retirado/archivado, el fallback es software y el recuento de tests es exactamente 257 (172 crate + 85 integración) a 2026-10-24.
- **Escenario 2: La guía de mapeo distingue vector y función en IWRAM**
  - *Dado:* `docs/guias/mapear-labels-aot.md`
  - *Cuando:* Se revisa el paso 4 sobre IWRAM
  - *Entonces:* `0x03007FFC` se documenta como puntero/vector de usuario y `0x030028A0` como código de `IntrMain`.
- **Escenario 3: La referencia de módulos de Rust no trunca descripciones**
  - *Dado:* `docs/referencia/api-modulos-rust.md`
  - *Cuando:* Se inspecciona la tabla de «Índice de Módulos»
  - *Entonces:* Ninguna celda termina con frases cortadas a la mitad ni con pipes sin escapar.
- **Escenario 4: Suite de pruebas en verde y Diátaxis sin errores**
  - *Dado:* El repositorio tras los cambios
  - *Cuando:* Se ejecutan `cargo test` y `validar_docs.py`
  - *Entonces:* Los 257 tests pasan y se obtienen 0 errores y 0 avisos.

---

### 6. Cómo lo comprobaré

- `cargo test`: 257 tests aprobados (172 crate + 85 integración).
- `python .agents/skills/diataxis-docs/scripts/validar_docs.py .`: 23 páginas revisadas, 0 errores, 0 avisos.
- `python scripts/extraer_referencias.py --check`: código 0.
- `python scripts/diagnostico/auditar_labels.py`: código 0.
- `python scripts/auditar_hallazgos.py`: código 0.
- `cargo test --test packaging_tests test_no_local_paths_in_packaging`: código 0.

---

## Resultado (lo escribe el agente tras verificar)

### Predicho vs. Real

| Aspecto | Predicho | Real | Veredicto |
| :--- | :--- | :--- | :---: |
| Estado del shim en `BACKLOG.md` | Retirar afirmaciones de que sigue en el repo y que es el fallback | Modificado en §3 y Prioridad #6 reflejando ADR 42 y contingencia software | Coincide |
| Fecha y tests en `BACKLOG.md` | Actualizar a 2026-10-24 y 257 tests (172 crate + 85 integración) | Cabecera y Estado del Proyecto actualizados con conteo exacto de 257 | Coincide |
| Naturaleza de Registro de Hallazgos | Explicar en cabecera que es histórico acumulativo de propuestas cerradas | Documentada nota explícita remitiendo a `PENDIENTES.md` para tareas abiertas | Coincide |
| Enlace a checklist en `AGENTS.md` | Apuntar a `AGENTS.md §7` en vez de `§6` | Enlace actualizado hacia `#checklist-obligatorio-de-pre-commit-gatekeeper` §7 | Coincide |
| Desambiguación de IWRAM | Clarificar rangos: combate en `0x030035C8`–`0x030036CC` e `IntrMain` en `0x030028A0` | Documentado en `mapear-labels-aot.md` paso 4 | Coincide |
| Puntero vs función de `IntrMain` | Explicar que `0x03007FFC` es el vector consultado por BIOS, no una función | Precisado en el paso 4 de `mapear-labels-aot.md` | Coincide |
| Procedimiento del blob IWRAM | Especificar arnés (`ppu_barrido_nativa`), volcado a `iwram_dump.bin` y pase a `recomp` | Añadido procedimiento operativo de 4 pasos en guía y `PENDIENTES.md` | Coincide |
| PPU software en arquitectura | Indicar que la producción es 100% GPU y que software es contingencia/diagnóstico | Actualizado en `arquitectura-port-android.md` (§ Alternativas #2) | Coincide |
| Backend preferente y calibración | Vulkan preferente (`false`) y calibración actualmente activa (`true`) | Rectificado en `medir-rendimiento.md` | Coincide |
| Símbolo `sub_8059E7C` y firmas | Aclarar que es nombre de `fireemblem7u` y retirar mención a firmas en TOML | Corregido en `anatomia-motor-fire-emblem-7.md` §1 y §7 | Coincide |
| Segunda biblioteca en APK | Nombrar `libblazing_recompiled.so` en empaquetado junto al runtime | Añadido a `compilar-y-empaquetar-apk.md` y `comandos-cli.md` | Coincide |
| Descripción de Hardware Shim | Reflejar Fase 1 sustituida y Fases 2-3 pendientes en catálogo | Actualizado en `docs/explicacion/README.md` | Coincide |
| Subtítulos Diátaxis | Eliminar prefijos `3.1` y `3.2` en `FORMATO_LABELS.md` | Normalizado a `### Cabecera` y `### Entradas de funciones` | Coincide |
| `examples/` y `telemetria/` en README | Incluir ambos directorios en estructura del proyecto | Añadidas entradas descriptivas en `README.md` | Coincide |
| Fuentes e Histórico en portal | Incorporar `fuentes/` e `historico/` en diagrama y texto de `docs/README.md` | Diagrama Mermaid y secciones del portal actualizados | Coincide |
| Extracción de docstrings y tabla | Evitar celdas cortadas y escapar pipes en `api-modulos-rust.md` | Función `resumen_doc_modulo` implementada y referencia regenerada limpia | Coincide |
| Suite de pruebas | 257 tests aprobados en verde | 257 tests aprobados (172 crate + 85 integración) | Coincide |
| Validación Diátaxis | 23 páginas, 0 errores, 0 avisos | `validar_docs.py`: 23 páginas revisadas, 0 errores, 0 avisos | Coincide |

### Contexto respetado

- Cero rutas absolutas locales en cualquier archivo (`test_no_local_paths_in_packaging` en verde).
- Sincronización continua de la referencia técnica de Rust (`extraer_referencias.py --check` en verde).
- Coherencia matemática e integridad de etiquetas (`auditar_labels.py` en verde).
- Registro acumulativo de hallazgos auditado (`auditar_hallazgos.py` en verde).

### Fuera de alcance (hallazgos que no se arreglaron)

- Ninguno: la totalidad de los 5 bloqueos y las 11 observaciones formuladas en la cuarta lectura de `RANT.MD` fueron resueltos en su alcance pactado.

### Revisión humana

- Los documentos corregidos quedan listos para una quinta lectura del auditor novato o la inspección directa del usuario. El tablero vivo (`BACKLOG.md`) y las guías prácticas operan ahora bajo una sola verdad técnica verificable.
