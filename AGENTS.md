# AGENTS.md — Blazing Recomp

Directrices, arquitectura y reglas operativas para agentes de IA que trabajen en el proyecto **Blazing Recomp**.

---

## 1. Visión y Propósito del Proyecto

- **Nombre:** Blazing Recomp (`blazing-recomp`)
- **Objetivo:** Port nativo del juego **Fire Emblem: The Blazing Blade (GBA)** con destino de ejecución nativa en **Android**.
- **Fundamento tecnológico:** Basado en el pipeline de recompilación estática AOT (*Ahead-Of-Time*) de `gba-recomp`:
  - Lógica del juego traducida a C11 y compilada nativamente para la arquitectura del dispositivo objetivo (**ARM64-v8a** en Android).
  - Runtime y modelo de hardware (PPU, APU, bus de memoria, DMA, timers) provistos por `gba-core` en Rust.
  - Cero emulación tradicional de la CPU en tiempo de ejecución.

---

## 2. Plataforma Objetivo: Android Nativo

1. **Arquitectura:**
   - Target principal: `aarch64-linux-android` (ARM64).
   - Secundario / Emuladores: `x86_64-linux-android`.
2. **Capa Nativa y Topología de Bibliotecas (.so en APK):**
   - El APK distribuye **dos bibliotecas dinámicas contiguas** en `lib/<abi>/`:
     - `libblazing_recomp.so`: Runtime en Rust generado por `cargo-ndk`. Es el punto de entrada oficial de la `NativeActivity` de Android (`android:hasCode="false"`, `android.app.lib_name="blazing_recomp"`). Controla el ciclo de vida, la GPU, el audio, el bus de memoria, DMA, timers y la capa de entrada.
     - `libblazing_recompiled.so`: Lógica C11 AOT del juego compilada de forma independiente por Clang del NDK a partir de las 12 unidades de `src-c/`. Contiene los 144.875 bloques C11 del juego y es cargada dinámicamente en tiempo de ejecución en Android por `src/recomp.rs` vía `dlopen("libblazing_recompiled.so")`.
3. **Renderizado & Pantalla:**
   - Pipeline gráfico de producción: **`NativePpu`** en GPU con shaders WGSL (`src/ppu_nativa.rs` y `src/present.rs`), con respaldo automático de Vulkan a OpenGL ES 3.0 a través de `wgpu`.
   - Conforme al ADR 42, el antiguo `PpuShim` por software y el tipo `Presenter` están completamente extintos. Prohibido sugerir o reintroducir emulación PPU por software como pipeline primario en producción. La rasterización por software en CPU provista por `gba-core` se mantiene exclusivamente como mecanismo de contingencia y diagnóstico (conmutable mediante la bandera `ppu-nativa-off.flag` o ante fallo en la inicialización de GPU), conforme a lo autorizado por el ADR 42.
   - Soporte para diferentes relaciones de aspecto y resoluciones de smartphones modernos (incluyendo escalado de píxeles nítido y posibles mejoras panorámicas).
4. **Controles & Entrada:**
   - Soporte para mandos físicos (Bluetooth y USB Type-C vía Gamepad API de Android).
   - Capa de controles táctiles virtuales superpuestos en pantalla (*touch overlay* personalizable).
   - Opcional: Integración de interacción táctil directa en los menús de batalla del juego si es viable.
5. **Rendimiento & Batería:**
   - La ejecución nativa debe optimizar el consumo de batería; sin bucles de sincronización activa (*busy-waiting*) innecesarios.
   - Pacing de fotogramas estricto a 60 FPS coordinado con el `Choreographer` de Android o VSync de la pantalla.

---

## 3. Datos del Juego y Seguridad Legal

- **ROM Oficial Pinneada:**
  - `Fire Emblem (Europe)` (16 MB)
  - SHA-256: `6f2c30823ab0237310b138739ccfad879f1a07c4a782a9cbb92317d60d03cee9`
- **BIOS Oficial Pinneada:**
  - `gba_bios.bin` (16 KB)
  - SHA-256: `fd2547724b505f487e6dcb29ec2ecff3af35a841a77ab2e85fd87350abd36570`
- **Regla Inviolable:**
  - **NUNCA** versionar ni incluir archivos `.gba`, `.bin` ni assets propietarios del juego en el repositorio.
  - La aplicación en Android debe incluir un flujo inicial donde el usuario selecciona su propia copia de la ROM y BIOS en el almacenamiento de su teléfono (verificando los hashes antes de permitir jugar).

---

## 4. Reglas Operativas para el Agente

1. **Verificación antes de modificar:**
   - Mantener siempre la independencia entre este repositorio (`blazing-recomp`) y el repositorio del motor base (`gba-recomp`).
   - Todo código específico de Android, wrappers JNI, layouts táctiles o empaquetado debe residir dentro de este repositorio.
2. **Enfoque de Cobertura (*0 Interpreter*):**
   - Priorizar la cobertura completa de funciones de *Fire Emblem* mediante el mapeo de labels (`blazing.labels.toml`).
   - El intérprete de respaldo solo debe actuar como red de estabilidad durante el desarrollo.
3. **Calidad de Código y Estilo:**
   - Código Rust limpio, idiomático y seguro.
   - Preservar comentarios arquitectónicos y decisiones de diseño no obvias.
   - Crear commits atómicos con mensajes descriptivos siguiendo *Conventional Commits* (`feat:`, `fix:`, `refactor:`, `docs:`).
4. **Topología Modular de Pruebas (`*_tests.rs` y `tests/`):**
   - **Tests Satélite:** Si un módulo en `src/` supera las 350 líneas de código o su bloque de pruebas unitarias supera las 150 líneas, los tests deben residir en un archivo satélite adyacente (`src/<modulo>_tests.rs`) vinculado con `#[cfg(test)] #[path = "<modulo>_tests.rs"] mod tests;`. Esto previene archivos monolíticos y preserva el acceso a structs y funciones privadas sin violar la encapsulación.
   - **Tests de Integración:** Todas las pruebas de empaquetado APK, manifiesto, compatibilidad de TV y sincronización de documentación residen exclusivamente en `tests/` aprovechando la configuración `crate-type = ["cdylib", "rlib"]`.
5. **Exactitud Matemática e Integridad de Tests:**
   - En cálculos de colisión y distancia euclidiana en coma flotante ($f32$), utilizar ternas pitagóricas exactas (ej. punto $(30.0, 40.0)$ con radio $50.0$) o deltas $\epsilon$ bien definidos para evitar falsos negativos por redondeo IEEE-754.
   - **Regla inviolable de mutación:** Nunca debilitar aserciones ni modificar código de producción para forzar el paso de un test o la muerte de un mutante. Si un mutante sobrevive, se debe crear un test de frontera más estricto.
6. **Restauración en Memoria en Scripts de Prueba y Auditoría:**
   - Todo script automatizado de mutación o estrés debe respaldar el contenido original de los archivos en memoria RAM y restaurarlo fielmente sobre el disco en un bloque `finally`.
   - **Prohibido:** Invocar comandos destructivos de Git (`git checkout`, `git restore`, `git reset`) dentro de scripts automatizados para evitar la pérdida silenciosa de trabajo en curso no commiteado.
7. **Higiene en Estructura Documental Diátaxis:**
   - En las páginas de referencia (`docs/referencia/`), utilizar estrictamente los encabezados estandarizados exigidos por `validar_docs.py` (ej. `## Descripción general`) sin prefijos numéricos que rompan la validación automática.
8. **Verificación de Línea Base Previa a Propuestas:**
   - Comprobar siempre que `cargo test` esté al 100% en verde **antes** de redactar cualquier propuesta o cambio de código, garantizando que el criterio de no regresión sea comprobable.
9. **Empaquetado de Máscaras de Bits sin Solapamiento:**
   - Al diseñar claves compuestas o tramos de empaquetado de bits (como `clave_de_tramo` o prioridades de capas/sprites), verificar que los campos no compartan bits para evitar colisiones silenciosas (*aliasing*).
10. **TDD en Contratos y Esquemas:**
    - Al diseñar formatos de telemetría o contratos de datos entre subsistemas, escribir y validar primero el test de esquema antes de implementar el formateador.
11. **Fuente Única de Verdad y Sincronización Transversal (Anti-Contradicciones):**
    - Al sustituir, refactorizar o retirar un componente, backend o flujo de ejecución (ej. sustitución de `PpuShim` por `NativePpu`, conmutación de backend gráfico por defecto, etc.):
      - **Prohibido:** Dejar afirmaciones contradictorias o desfasadas en guías prácticas (`docs/guias/`), explicaciones o en el cuerpo narrativo de `docs/BACKLOG.md`.
      - **Obligatorio:** Realizar un escaneo transversal (`git grep`) y alinear todos los documentos satélites (`README.md`, `GLOSSARY.md`, `docs/`) para que reflejen de forma unánime y consistente la misma realidad técnica de producción.
12. **Higiene en Tableros Vivos (`BACKLOG.md` vs `PENDIENTES.md`):**
    - Para prevenir la acumulación desbordada de viñetas que desorienten al lector:
      - Todo hallazgo en el "Registro de Hallazgos" de `docs/BACKLOG.md` debe consignar explícitamente su estado (`[Abierto]` | `[Cerrado: AAAA-MM-DD]`).
      - Si una tarea o hallazgo ha sido resuelto por una propuesta posterior, el agente debe marcarlo inmediatamente como cerrado o retirarlo de la lista activa.
      - `docs/PENDIENTES.md` se reserva de forma estricta para tareas de desarrollo reales y accionables en código.
13. **Precisión Topográfica en Memoria y Hardware de Bajo Nivel:**
    - En documentación técnica, guías de desensamblado o mapas AOT:
      - **Diferenciación de punteros y código:** Diferenciar siempre de forma inequívoca una dirección que aloja un vector o puntero de memoria (ej. `0x03007FFC`, User IRQ vector) del punto de entrada ejecutable de la subrutina (`0x030028A0`, `IntrMain`).
      - **Diferenciación de buffers y subrutinas:** No confundir buffers de datos de combate (ej. `0x03002B80`) con rutinas ejecutables de interpolación (`0x030035C8`).
      - **Fidelidad de metadatos:** No atribuir a esquemas de configuración (como `blazing.labels.toml`) propiedades que no poseen (como firmas o tipos de retorno).
14. **Prohibición de Afirmaciones Volátiles en Texto Rígido (Anti-Bitrot):**
    - **Prohibido:** Escribir en texto narrativo recuentos numéricos volátiles que cambian con frecuencia (ej. *"la suite tiene exactamente 250 tests"* o *"se han catalogado 18.483 funciones"*), salvo que vayan acompañados de la fecha exacta de corte o estén validados por tests de integración que garanticen su actualización.
    - Siempre que se describa el estado de pruebas o etiquetas, remitir al comando de inspección dinámico oficial (`cargo test -- --list`, `auditar_labels.py`).

### Topología de Repositorios, Motor Upstream y Delimitación de Archivos
Este proyecto depende de los componentes y crates del motor base del proyecto oficial `gba-recomp` de Jack Rickey:
- **Port de Fire Emblem (este repositorio):** `blazing-recomp/` (el paquete Cargo se denomina `blazing-recomp`).
- **Motor Upstream Oficial:** [`JRickey/gba-recomp`](https://github.com/JRickey/gba-recomp)
  - Motor de hardware y bus de GBA: `crates/gba-core`
  - Decodificador de instrucciones ARMv4T: `crates/armv4t`
  - Shaders y renderizado GPU: `crates/screen`
- **Delimitación de `pack.toml`:** `pack.toml` es un manifiesto consumido exclusivamente por el generador de código estático upstream (`recomp build`). El runtime en Rust y los scripts de empaquetado (`scripts/package_apk.ps1`) **NO leen este archivo en tiempo de ejecución**.
- **Herramienta `recomp`:** Es el ejecutable CLI provisto por `JRickey/gba-recomp`. Comandos como `cargo build --release -p recomp` solo existen y deben ejecutarse dentro del workspace clonado de `gba-recomp`.
- **Estado de dependencias y directiva `[patch]`:** Aunque las dependencias base en `Cargo.toml` declaran la URL del repositorio Git oficial fijadas a un commit inmutable (`rev = "de4edf59b872d887046d6a3b005e2df551b6d44c"`), actualmente el archivo `Cargo.toml` mantiene **activa de forma requerida** la directiva `[patch."https://github.com/JRickey/gba-recomp.git"]` apuntando a la carpeta hermana `../gba-recomp/crates/gba-core`. Esto se debe a que extensiones críticas de telemetría de video (`perf-video`), contadores de rendimiento y la omisión de PPU por software residen en dicho árbol local en proceso de integración en el upstream oficial. Por tanto, en el estado actual de desarrollo, el repositorio **requiere la presencia local de `../gba-recomp` para compilar `cargo test`, el visor interactivo y el APK**. La autonomía total sin carpetas hermanas locales representa el objetivo de entrega de la versión de release, no el estado del árbol de trabajo actual.


## 5. Skills del Repositorio (`.agents/skills/`)

El repositorio cuenta con skills locales en [`.agents/skills/`](.agents/skills/) que los agentes deben consultar y aplicar rigurosamente según la tarea solicitada:

### 1. `propuesta-verificable`
- **Definición:** [`.agents/skills/propuesta-verificable/SKILL.md`](.agents/skills/propuesta-verificable/SKILL.md)
- **Propósito:** Flujo estricto de propuesta antes de editar código. El agente predice el cambio exacto (con ejemplos "antes / ahora" o criterios Dado/Cuando/Entonces), espera la aprobación explícita del usuario, edita únicamente lo autorizado y valida el resultado contrastando lo predicho contra lo real.
- **Niveles de detalle:**
  - **Nivel 0:** Cambios triviales de texto, formato o comentarios (edición directa sin propuesta previa).
  - **Nivel 1:** Cambios acotados a un solo archivo sin riesgo sistémico (propuesta concisa en chat con antes/ahora, test o verificación y comando).
  - **Nivel 2:** Modificación de más de 3 archivos, creación/borrado de archivos, cambios en lógica compartida, módulos nuevos o zonas sensibles (requiere crear una propuesta formal en `propuestas/AAAA-MM-DD_nombre.md` y esperar aprobación antes de tocar código).
- **Activación:** Siempre que se vaya a modificar, crear o borrar código del proyecto, refactorizar, agregar funciones o modificar la configuración de compilación del port. *No aplica para consultas teóricas o explicaciones.*

### 2. `diataxis-docs`
- **Definición:** [`.agents/skills/diataxis-docs/SKILL.md`](.agents/skills/diataxis-docs/SKILL.md)
- **Propósito:** Diseño y mantenimiento de documentación técnica en español bajo el estándar Diátaxis, manteniendo estricta separación entre los cuatro cuadrantes para no mezclar necesidades del lector.
- **Estructura en `docs/`:**
  - **Tutoriales (`docs/tutoriales/`):** Aprendizaje guiado paso a paso para recién llegados.
    *(Cuadrante aún vacío: el directorio no existe todavía. El `<nav>` del validador Diátaxis no lo
    enlaza, así que cualquier página que se añada ahí debe nacer con su propio índice enlazado.
    No se crea la carpeta vacía sin contenido.)*
  - **Guías prácticas (`docs/guias/`):** Pasos directos para tareas concretas (ej. compilación cruzada con NDK, integración JNI, generación del APK).
  - **Referencia ([`docs/referencia/`](docs/referencia/README.md)):** Especificaciones técnicas puras (APIs nativas, flags de `pack.toml`, comandos).
  - **Explicación ([`docs/explicacion/`](docs/explicacion/README.md)):** Detalla **cómo funciona este proyecto por dentro**: arquitectura interna del port Android, diseño de software, compromisos técnicos (*trade-offs*), lecciones aprendidas sobre nuestro propio código y el registro histórico de decisiones de ingeniería ([`DECISIONES.md`](docs/explicacion/DECISIONES.md)).
  - **Propuestas ([`docs/propuestas/`](docs/propuestas/README.md)):** Propuestas formales de Nivel 2
    (no es un cuadrante Diátaxis: es el registro del flujo `propuesta-verificable`). Cada archivo
    declara su estado en el bloque `>` de cabecera (`Estado:` / `Resultado:`).
  - **Prototipos (`docs/prototipos/`):** Bocetos HTML de trabajo referenciados desde las propuestas
    de overlay; material efímero, no documentación Diátaxis versionada como página de lectura.
  - **Fuentes ([`docs/fuentes/`](docs/fuentes/README.md)):** Repositorio de consulta de **conocimiento externo y especificaciones técnicas objetivas**: física y arquitectura del hardware de Game Boy Advance, ISA ARMv4T/AArch32, funcionamiento de GPUs móviles TBDR/UMA, subsistemas de audio de bajo nivel (AAudio / MMAP) y almacenamiento en Android (SAF / Scoped Storage). Representa la fuente de verdad técnica externa que agentes y desarrolladores consultan para fundamentar las decisiones de diseño. **Regla estricta:** Es de solo lectura para los agentes de IA, a menos que el desarrollador humano solicite explícitamente redactar o agregar una nueva fuente. No forma parte de los cuatro cuadrantes Diátaxis editables.
- **Activación:** Siempre que se solicite documentar el proyecto, crear guías de instalación o despliegue para Android, documentar interfaces nativas, actualizar docs tras cambios de código o estructurar la carpeta `docs/`.

### 3. `explicar-codigo-documento`
- **Definición:** [`.agents/skills/explicar-codigo-documento/SKILL.md`](.agents/skills/explicar-codigo-documento/SKILL.md)
- **Propósito:** Creación de guías de estudio didácticas paso a paso en Markdown, estructuradas con estilo *rubber-duck* y momentos interactivos "Tu turno" para que el usuario explique y asimile el código antes de ver la respuesta.
- **Activación:** Siempre que el usuario solicite estudiar o entender el código, generar guías de aprendizaje, apuntes técnicos de un módulo o diga "explícame cómo funciona esto".

### 4. `test-generation`
- **Definición:** [`.agents/skills/test-generation/SKILL.md`](.agents/skills/test-generation/SKILL.md)
- **Propósito:** Generación, mejora y auditoría de tests (unitarios, de integración y de mutación) orientados a detectar fallos reales sin inflar métricas artificiales de cobertura ni alterar el código de producción.
- **Principios y tipos:**
  - **Unitarios:** Lógica pura y funciones aisladas con aserciones concretas y deterministas.
  - **Integración:** Conexión e interoperabilidad entre módulos y subsistemas.
  - **Mutación:** Evaluación de la efectividad y resistencia de la suite de pruebas mediante mutantes.
- **Activación:** Siempre que se solicite crear, ampliar o auditar pruebas, mejorar la cobertura de tests, diseñar fixtures o evaluar la solidez del sistema ante cambios.

### 5. `lenguaje-claro-iso-24495` (Directriz Transversal de Comunicación)
- **Definición:** [`.agents/skills/lenguaje-claro-iso-24495/SKILL.md`](.agents/skills/lenguaje-claro-iso-24495/SKILL.md)
- **Propósito:** Garantizar que todas las comunicaciones, explicaciones, propuestas, respuestas en el chat y textos en español cumplan los principios de la norma ISO 24495-1 / UNE-ISO 24495-1: contenido pertinente, fácil de encontrar, fácil de comprender y fácil de usar.
- **Regla transversal:** Aplica de forma obligatoria y predeterminada a toda interacción del agente:
  - Ir directo al grano al inicio, sin fórmulas de relleno ni rodeos.
  - Explicar términos técnicos solo cuando sea necesario para la persona lectora, preservando la exactitud del dominio.
  - Ordenar cronológicamente las acciones y utilizar listas o tablas para facilitar la lectura visual rápida.
  - Mantener la precisión técnica y el rigor sin sacrificar la claridad.
- **Activación:** Siempre activa en todas las interacciones, propuestas, reportes y redacción en español.

### 6. Pipeline Operativo al Modificar Código

Todo agente que vaya a modificar, crear o refactorizar código en este repositorio debe ejecutar de forma obligatoria y secuencial este pipeline de 5 fases, encadenando las skills locales:

```
[Solicitud de Cambio]
         │
         ▼
 1. Diagnóstico & Tests Previos ──► test-generation (estrategia temporal) + propuesta-verificable (nivel y alcance)
         │
         ▼
 2. Aprobación (Gatekeeper)     ──► Esperar "OK" explícito del usuario (Nivel 1 o 2)
         │
         ▼
 3. Edición Atómica             ──► Aplicar estrictamente el cambio acordado (Cero refactors no pactados)
         │
         ▼
 4. Verificación & Robustez     ──► test-generation (ejecutar suite + edge cases + mutación)
         │
         ▼
 5. Sincronización & Cierre     ──► diataxis-docs + registro de "Fuera de alcance" + reporte "Predicho vs. Real"
```

#### Fase 1: Diagnóstico y Estrategia de Tests
1. **Clasificar el objetivo:** Determinar si es un **Bugfix**, **Refactor** o **Nueva Feature**.
2. **Definir el test previo (`test-generation`):**
   - *Bugfix:* Diseñar el test de reproducción. **Debe fallar en rojo** antes de tocar el código de producción.
   - *Refactor:* Verificar que la suite actual pase **en verde** antes de cambiar nada (línea base).
   - *Feature:* Diseñar el test de contrato/especificación (estilo TDD).
3. **Clasificar el nivel (`propuesta-verificable`):** Determinar Nivel 0 (trivial/texto), Nivel 1 (acotado a 1 archivo) o Nivel 2 (crítico, nuevo módulo o >3 archivos). Delimitar explícitamente en el alcance qué **No incluye** el cambio.

#### Fase 2: Propuesta y Gatekeeper
- En **Nivel 1:** Presentar en el chat el "Antes / Ahora", comando de verificación y tests esperados.
- En **Nivel 2:** Redactar la propuesta formal en `docs/propuestas/AAAA-MM-DD_nombre.md` (o `propuestas/`).
- **Regla estricta:** En Niveles 1 y 2 está **prohibido modificar código** sin la confirmación explícita del usuario.

#### Fase 3: Edición Atómica
- Modificar únicamente lo autorizado en la propuesta.
- **Manejo de hallazgos colaterales:** Si durante la edición se detectan bugs secundarios, código muerto o tentaciones de refactorizar piezas no acordadas:
  - **Prohibido:** Aplicar "refactorizaciones oportunistas" no autorizadas dentro de la misma tarea.
  - **Obligatorio:** Anotar el hallazgo para registrarlo en el cierre sin desviar el alcance acordado.

#### Fase 4: Verificación y Robustez
1. **Ejecutar tests:** Comprobar que los tests previos ahora pasen en verde.
2. **Blindaje (`test-generation`):** Añadir tests de casos límite (nulos, vacíos, desbordamientos, límites numéricos).
3. **Comprobación de solidez:** Verificar con mutación o alterando deliberadamente una condición del código para asegurar que los tests fallen si la lógica se altera.
4. **Regla estricta:** Nunca debilitar asserts ni modificar código de producción solo para forzar el paso de un test.

#### Fase 5: Sincronización Documental y Cierre
1. **Impacto en documentación (`diataxis-docs`):** Si el cambio altera APIs nativas, flags, arquitectura o guías de compilación Android, actualizar el cuadrante correspondiente en `docs/`.
2. **Regeneración y Validación Automática de Tableros (Obligatorio):**
   - Ejecutar obligatoriamente `python scripts/actualizar_indice_propuestas.py` para sincronizar y reindexar `docs/BACKLOG.md`.
   - Ejecutar `python .agents/skills/diataxis-docs/scripts/validar_docs.py` verificando 0 errores y 0 advertencias.
3. **Higiene Activa de Tableros (`BACKLOG.md` y `PENDIENTES.md`):**
   - Si la tarea resolvió un hallazgo registrado en `docs/BACKLOG.md`, marcarlo inmediatamente como cerrado con fecha (`[Cerrado: AAAA-MM-DD]`).
   - Si resolvió una tarea de `docs/PENDIENTES.md`, marcarla como completada (`[x]`).
   - Sincronizar fecha y versión en la cabecera del `BACKLOG.md`.
4. **Gatekeeper de Decisiones Arquitectónicas (`docs/explicacion/DECISIONES.md`):**
   - Pregunta obligatoria de auditoría: *¿Este cambio introduce una nueva directriz arquitectónica, sustituye o altera un ADR previo, o formaliza un compromiso técnico estructural?*
   - Si la respuesta es afirmativa, redactar obligatoriamente el siguiente ADR consecutivo en `docs/explicacion/DECISIONES.md`, clasificar su vigencia (`[Vigente]`, `[Sustituye a...]`) y añadirlo en el índice cronológico.
5. **Registro de hallazgos fuera de alcance y tareas pendientes:** Registrar obligatoriamente todo bug, refactor o deuda técnica detectada durante la edición en la sección de la propuesta:
   `### Fuera de alcance (hallazgos que no se arreglaron)` con su nivel de impacto (`bajo | medio`), o en el resumen de cierre del chat (en Nivel 1).
   - **Regla estricta de filtrado (`docs/PENDIENTES.md` vs `docs/BACKLOG.md`):** Al completar cada propuesta, los hallazgos que constituyan **tareas pendientes reales/accionables** (código por escribir, refactors necesarios, funciones por implementar o bugs reales) deben registrarse o actualizarse en [`docs/PENDIENTES.md`](docs/PENDIENTES.md). Los aprendizajes técnicos, notas de diseño o limitaciones físicas de hardware sin acción de código asociada deben conservarse únicamente en [`docs/BACKLOG.md`](docs/BACKLOG.md) y [`docs/explicacion/DECISIONES.md`](docs/explicacion/DECISIONES.md).
6. **Reporte final:** Contrastar lo predicho en la propuesta contra el resultado real obtenido (`Predicho vs. real`).

---

## 7. Notas y Reglas Personalizadas del Usuario

*(Espacio reservado para preferencias, decisiones de diseño y restricciones añadidas por el usuario durante el desarrollo)*

- **Portabilidad y Privacidad (Prohibición de rutas locales absolutas):**
  - **NUNCA** escribir ni versionar rutas absolutas locales del dispositivo del usuario (ej. `C:\Users\...`, `file:///C:/...` o rutas que contengan el nombre de usuario o rutas de sistema) dentro de ningún archivo del repositorio (`.md`, `.toml`, código fuente o scripts).
  - Todas las referencias y enlaces a archivos dentro del proyecto deben ser **estrictamente relativos al repositorio** (ej. `docs/BACKLOG.md`, `.agents/skills/`, `../src-c/`).
  - Esto garantiza que el repositorio sea 100% portable, funcione idénticamente en cualquier máquina o sistema operativo y no exponga datos de privacidad del entorno local si se publica en GitHub.

- **Comando Oficial de Empaquetado de APK:**
  - Todo empaquetado, compilación multi-ABI y generación del instalador APK para Android debe ejecutarse obligatoriamente con el comando exacto:
    `powershell -ExecutionPolicy Bypass -File scripts/package_apk.ps1`
  - Este script compila automáticamente los tres ABIs soportados (`arm64-v8a`, `armeabi-v7a`, `x86_64`) con `cargo ndk`, procesa recursos con `aapt2`, alinea memoria a 16 KB con `zipalign` y firma el Fat APK con `apksigner` bajo los esquemas v1/v2/v3 sin depender de Gradle.
  - Para verificar que el APK se genera limpiamente ante cambios en el manifiesto, dependencias nativas o integración de assets, el agente debe invocar siempre esta orden completa.

- **Suite Oficial de Diagnóstico e Ingeniería Inversa AOT (`scripts/diagnostico/`):**
  Para investigar cuellos de botella de rendimiento, analizar telemetría emitida por el motor en Android o mapear nuevas subrutinas hacia código nativo C11, los agentes deben utilizar las herramientas modulares del proyecto:
  1. **Análisis de Telemetría:**
     `python scripts/diagnostico/analizar_telemetria.py --archivo <log.txt> --top <N>`
     Calcula tasas de FPS, desglose por fases de frame (`ejec`, `ppu`, `apu`, `dibujo`, `present`) y genera el ranking porcentual de hotspots del intérprete.
  2. **Desensamblado e Inspección de la ROM:**
     `python scripts/diagnostico/desensamblar_rom.py --addr <0x08xxxxxx> --count <N>`
     `python scripts/diagnostico/desensamblar_rom.py --buscar-prologo <0x08xxxxxx>`
     Decodifica instrucciones Thumb de la ROM oficial y localiza automáticamente los puntos de entrada canónicos (`PUSH {..., lr}`, `0xb5xx`) o destinos de *jump tables* (`mov pc, rx`).
  3. **Auditoría de Labels AOT:**
     `python scripts/diagnostico/auditar_labels.py`
     Comprueba la integridad de [`blazing.labels.toml`](blazing.labels.toml) frente a la ROM, verificando que no existan nombres duplicados, colisiones de dirección ni rangos inválidos.
  4. **Generador AOT C11 Automatizado:**
     `powershell -ExecutionPolicy Bypass -File scripts/diagnostico/compilar_recomp_aot.ps1`
     Ejecuta `recomp build` con `$env:RECOMP_KEEP_C = "1"` y sincroniza de forma inmediata los archivos C11 generados en `src-c/`.

- **Checklist Obligatorio de Pre-Commit Gatekeeper:**
  Antes de proponer o ejecutar un `git commit` y `git push`, el agente debe verificar sin excepción los siguientes 5 criterios:
  1. **Suite de pruebas al 100% en verde:** `cargo test` ejecutado y aprobado (todos los tests unitarios y de integración pasando sin fallos).
  2. **Validación Diátaxis sin errores:** `python .agents/skills/diataxis-docs/scripts/validar_docs.py .` devuelve `0 error(es), 0 aviso(s)`.
  3. **Especificaciones de API sincronizadas:** `python scripts/extraer_referencias.py --check` sale con código de éxito 0.
  4. **Auditoría de labels AOT aprobada:** `python scripts/diagnostico/auditar_labels.py` sale con código de éxito 0.
  5. **Auditoría de rutas absolutas limpia:** Ningún archivo agregado o modificado contiene rutas locales del usuario o del sistema.

