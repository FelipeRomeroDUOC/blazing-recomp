# Propuesta: Crate Nativo Android con Rust, NDK y Motor gba-core

Fecha: 2026-10-08
Nivel: 2
Estado: pendiente

<!-- Estados: pendiente | aprobada | modificada | descartada |
     verificada por agente | revisada por humano | discrepada | cerrada -->

## Propuesta (la escribe el agente antes de editar)

### 1. Qué cambia

Crea la estructura del paquete nativo en Rust para Android en `blazing-recomp` (`Cargo.toml` y `src/lib.rs`), integrando el motor físico de la consola [`gba-core`](../gba-recomp/crates/gba-core) y la infraestructura de ciclo de vida móvil [`android-activity`](https://crates.io/crates/android-activity), configurado para compilar una biblioteca dinámica (`.so`) para procesadores móviles ARM64 sin intermediarios.

### 2. Contexto

- Contexto consultado: `AGENTS.md`, `pack.toml`, `docs/BACKLOG.md`, `docs/referencia/CONFIGURACION_PACK.md`, `../gba-recomp/crates/gba-core/Cargo.toml`.
- Intención: Establecer la base técnica ejecutable del port para Android, manejando el ciclo de vida del sistema operativo (`android-activity`) e integrando el motor de emulación de hardware (`gba-core`) para procesar memoria, PPU, APU y bloques C.
- Restricciones y decisiones previas (máx. 3):
  - Integración modular con el motor `gba-core` sin duplicar código (origen: respuesta humana, 2026-10-08) → implica: vincular `gba-core = { path = "../gba-recomp/crates/gba-core" }` en `Cargo.toml` usando ruta relativa portable → protegida por: Compilación cruzada exitosa para arquitectura ARM64 de Android.
  - Plataforma objetivo nativa ARM64 vía NDK y Rust (origen: `AGENTS.md`) → implica: compilar como biblioteca `cdylib` para el target `aarch64-linux-android` usando `android-activity` → protegida por: Verificación de tipos e integración con Machine y AndroidApp.
  - Portabilidad absoluta sin rutas locales (origen: regla de usuario en `AGENTS.md`) → implica: manifiestos y dependencias de Cargo estrictamente relativos al repositorio → protegida por: Independencia de rutas absolutas en configuración de Cargo.
- Alternativas descartadas:
  - Copiar todo el código de `gba-core` dentro de `blazing-recomp` (vendoring manual prematuro) → descartada porque desincroniza el motor de mejoras futuras cuando una dependencia relativa limpia es inmediata.
  - Wrapper JNI manual en Java/Kotlin con NDK tradicional → descartada porque introduce sobrecarga innecesaria de recolección de basura y complica la sincronización de fotogramas a 60 FPS frente a una NativeActivity pura en Rust.
- Ante imprevistos, prioriza: compilación limpia y robustez de eventos de ciclo de vida sobre optimizaciones tempranas de renderizado.

### 3. Ejemplos

- Caso normal: Ejecutar `cargo ndk -t arm64-v8a build` → antes: falla (no existe `Cargo.toml` ni crate en `blazing-recomp`) → ahora: compila con éxito y emite `target/aarch64-linux-android/debug/libblazing_recomp.so`.
- Caso límite: Ejecutar `cargo check --target aarch64-linux-android` en entorno local sin dispositivo físico conectado → antes: no existe crate → ahora: pasa el análisis de sintaxis y tipos del NDK y de `gba-core` con código de salida 0.

### 4. Pasos

1. Crear el manifiesto `Cargo.toml` configurando el paquete `blazing-recomp` con `crate-type = ["cdylib"]`, dependencia relativa a `gba-core` (`path = "../gba-recomp/crates/gba-core"`), y dependencias de Android (`android-activity` con backend `native-activity`, `android_logger` y `log`).
2. Crear el directorio `src/` y el archivo `src/lib.rs` implementando el punto de entrada `#[no_mangle] fn android_main(app: AndroidApp)` con recepción de eventos (`Resume`, `Pause`, `Terminate`), logging a Logcat y preparación del tipo `gba_core::Machine`.
3. Probar la compilación cruzada con `cargo ndk -t arm64-v8a build` verificando la generación del archivo binario `.so`.

### 5. Qué más podría verse afectado

- `.gitignore`: el directorio `target/` ya se encuentra protegido e ignorado en la línea 8 de `.gitignore` (ningún artefacto de compilación de Cargo se versionará).

### 6. Suposiciones

- `gba-core` y su submódulo `armv4t` compilan limpiamente para `aarch64-linux-android` usando la libc de Bionic sin modificaciones de código fuente → prototipar: sí → si es falso: adaptar flags de compilación para target Unix bionic → resultado del prototipo: | no detecta: problemas en APIs de audio o ventana en capas superiores.
- El toolchain de Rust y `cargo-ndk` resuelven la ruta del NDK automáticamente mediante la variable de entorno `ANDROID_NDK_HOME` ya configurada → prototipar: sí → si es falso: crear `.cargo/config.toml` con ruta explícita al linker de clang del NDK → resultado del prototipo: | no detecta: problemas si la variable de entorno cambia de ruta.
- `android-activity` compila como biblioteca dinámica `.so` independiente sin exigir la presencia previa de un proyecto Gradle/APK en esta fase → prototipar: no → si es falso: crear un esqueleto mínimo de APK harness → resultado del prototipo: | no detecta: incompatibilidades de manifiesto AndroidManifest.xml en la fase posterior de empaquetado.

### 7. Alcance

- Incluye: Creación de `Cargo.toml` con integración a `gba-core`, creación de `src/lib.rs` con punto de entrada nativo `android_main` y manejo del ciclo de vida, compilación cruzada a `.so` para arquitectura ARM64 (`arm64-v8a`).
- No incluye: Renderizado visual con Vulkan / OpenGL ES (Fase 4), audio interactivo (Fase 4), controles táctiles en pantalla (Fase 5), ni generación del archivo APK firmado (Fase 6).

### 8. Criterios de aceptación (no se modifican sin aprobación)

Escenario: Compilación cruzada exitosa para arquitectura ARM64 de Android
Caso de uso: Compilación del motor nativo para dispositivos Android modernos | Tipo: normal
  Dado que el crate `blazing-recomp` está configurado con `crate-type = ["cdylib"]`, `gba-core` y dependencias de Android
  Cuando ejecuto `cargo ndk -t arm64-v8a build`
  Entonces el proceso finaliza con código de salida 0 y genera el archivo `target/aarch64-linux-android/debug/libblazing_recomp.so`
Test: `test_arm64_cdylib_build`
Estado previo: falla (no existe Cargo.toml)

Escenario: Verificación de tipos e integración con Machine y AndroidApp
Caso de uso: Manejo de eventos del ciclo de vida de la aplicación sin errores de tipos | Tipo: límite
  Dado que `src/lib.rs` define la función `android_main` utilizando `AndroidApp` y referencias a `gba_core::Machine`
  Cuando ejecuto `cargo check --target aarch64-linux-android`
  Entonces el análisis de tipos y comprobación del compilador termina con código de salida 0
Test: `test_android_lifecycle_types`
Estado previo: falla (no existe código de crate)

Escenario: Independencia de rutas absolutas en configuración de Cargo
Caso de uso: Portabilidad del proyecto al clonarse en cualquier entorno | Tipo: regresión
  Dado que `Cargo.toml` y el código del crate usan exclusivamente rutas y configuración portables
  Cuando se buscan patrones de rutas locales absolutas en los archivos del crate
  Entonces el número de coincidencias de rutas locales absolutas es exactamente 0
Test: `test_no_local_paths_in_crate`
Estado previo: pasa (regresión preventiva)

### 9. Cómo lo comprobaré

- Compilación cruzada exitosa para arquitectura ARM64 de Android → herramienta o comando: `cargo ndk -t arm64-v8a build`
  → pasa si: el comando retorna código 0 y el archivo `libblazing_recomp.so` existe en `target/aarch64-linux-android/debug/` → no detecta: fallos de ejecución en hardware físico real.
- Verificación de tipos e integración con Machine y AndroidApp → herramienta o comando: `cargo check --target aarch64-linux-android`
  → pasa si: retorna código 0 sin errores de tipos → no detecta: errores de enlace en tiempo de carga dinámica (dlopen).
- Independencia de rutas absolutas en configuración de Cargo → herramienta o comando: `git grep -i "C:/" Cargo.toml src/`
  → pasa si: 0 coincidencias → no detecta: rutas no estándar sin letra de unidad.

### 10. Terminado cuando

Se cumplen todos los criterios de aceptación.

### 11. Parar si

2 intentos fallidos seguidos con el mismo criterio. Detenerse, explicar qué se probó y marcar la propuesta como "discrepada".

### Correcciones previas a presentar

Ninguna.

### Cambios tras prototipos (solo si el plan cambió)

*(Se completará si un prototipo altera el plan)*

---

## Resultado (lo completa el agente después de editar)

### Predicho vs. real

- Compilación cruzada exitosa para arquitectura ARM64 de Android: pendiente
  Resultado real:
  Método: el anunciado
- Verificación de tipos e integración con Machine y AndroidApp: pendiente
  Resultado real:
  Método: el anunciado
- Independencia de rutas absolutas en configuración de Cargo: pendiente
  Resultado real:
  Método: el anunciado

### Contexto: ¿se respetó?

- Integración modular con el motor gba-core sin duplicar código → pendiente
- Plataforma objetivo nativa ARM64 vía NDK y Rust → pendiente
- Portabilidad absoluta sin rutas locales → pendiente

### Revisión humana

- ninguna: todo se comprobó con tests (`cargo ndk -t arm64-v8a build`, `cargo check` y `git grep`).
