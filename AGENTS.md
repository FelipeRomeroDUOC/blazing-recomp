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
2. **Capa Nativa (NDK & Rust):**
   - El código Rust y C recompilado se empaquetará como biblioteca compartida (`.so`) enlazada vía JNI o `android-activity` / `winit`.
3. **Renderizado & Pantalla:**
   - Backend gráfico compatible con Android: **Vulkan** o **OpenGL ES 3.0** a través de `wgpu`.
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

### Topología de Repositorios y Motor Upstream
Este proyecto depende de los componentes y crates del motor base del proyecto oficial `gba-recomp` de Jack Rickey:
- **Port de Fire Emblem (este repositorio):** `blazing-recomp/`
- **Motor Upstream Oficial:** [`JRickey/gba-recomp`](https://github.com/JRickey/gba-recomp)
  - Motor de hardware y bus de GBA: `crates/gba-core`
  - Decodificador de instrucciones ARMv4T: `crates/armv4t`
  - Shaders y renderizado GPU: `crates/screen`
- En `Cargo.toml`, las dependencias se vinculan directamente al repositorio Git oficial fijadas a un commit SHA inmutable (`rev = "de4edf59b872d887046d6a3b005e2df551b6d44c"`), garantizando que el repositorio sea 100% autónomo, portable y reproducible sin depender de carpetas hermanas locales en el sistema de archivos.
- Para desarrollo y pruebas locales del motor en paralelo, se puede recurrir temporalmente a la directiva `[patch."https://github.com/JRickey/gba-recomp.git"]` apuntando a `../gba-recomp`.


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
  - **Guías prácticas (`docs/guias/`):** Pasos directos para tareas concretas (ej. compilación cruzada con NDK, integración JNI, generación del APK).
  - **Referencia ([`docs/referencia/`](docs/referencia/README.md)):** Especificaciones técnicas puras (APIs nativas, flags de `pack.toml`, comandos).
  - **Explicación ([`docs/explicacion/`](docs/explicacion/DECISIONES.md)):** Decisiones de diseño, arquitectura del port Android y compromisos técnicos.
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

### 5. Pipeline Operativo al Modificar Código

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
2. **Registro de hallazgos fuera de alcance:** Registrar obligatoriamente todo bug o refactor detectado durante la edición en la sección de la propuesta:
   `### Fuera de alcance (hallazgos que no se arreglaron)` con su nivel de impacto (`bajo | medio`), o en el resumen de cierre del chat (en Nivel 1) para alimentar futuras propuestas.
3. **Reporte final:** Contrastar lo predicho en la propuesta contra el resultado real obtenido (`Predicho vs. real`).

---

## 6. Notas y Reglas Personalizadas del Usuario

*(Espacio reservado para preferencias, decisiones de diseño y restricciones añadidas por el usuario durante el desarrollo)*

- **Portabilidad y Privacidad (Prohibición de rutas locales absolutas):**
  - **NUNCA** escribir ni versionar rutas absolutas locales del dispositivo del usuario (ej. `C:\Users\...`, `file:///C:/...` o rutas que contengan el nombre de usuario o rutas de sistema) dentro de ningún archivo del repositorio (`.md`, `.toml`, código fuente o scripts).
  - Todas las referencias y enlaces a archivos dentro del proyecto deben ser **estrictamente relativos al repositorio** (ej. `docs/BACKLOG.md`, `.agents/skills/`, `../src-c/`).
  - Esto garantiza que el repositorio sea 100% portable, funcione idénticamente en cualquier máquina o sistema operativo y no exponga datos de privacidad del entorno local si se publica en GitHub.

- **Checklist Obligatorio de Pre-Commit Gatekeeper:**
  Antes de proponer o ejecutar un `git commit` y `git push`, el agente debe verificar sin excepción los siguientes 4 criterios:
  1. **Suite de pruebas al 100% en verde:** `cargo test` ejecutado y aprobado (49 tests actuales: 41 en `src/lib.rs` + 8 en `tests/packaging_tests.rs`).
  2. **Validación Diátaxis sin errores:** `python .agents/skills/diataxis-docs/scripts/validar_docs.py .` devuelve `0 error(es), 0 aviso(s)`.
  3. **Especificaciones de API sincronizadas:** `python scripts/extraer_referencias.py --check` sale con código de éxito 0.
  4. **Auditoría de rutas absolutas limpia:** Ningún archivo agregado o modificado contiene rutas locales del usuario o del sistema.
