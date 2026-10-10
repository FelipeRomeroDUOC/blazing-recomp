# Propuesta: compatibilidad-android-tv-y-armeabi-v7a

Fecha: 2026-10-09
Nivel: 2
Estado: revisada por humano

<!-- Estados: pendiente | aprobada | modificada | descartada |
     verificada por agente | revisada por humano | discrepada | cerrada -->

## Propuesta (la escribe el agente antes de editar)

### 1. Qué cambia

La aplicación podrá instalarse y ejecutarse en dispositivos Android TV (Android 10 y similares) sin ser rechazada por el instalador del sistema por ausencia de pantalla táctil o arquitectura no coincidente, habilitando firmas v1/v2/v3 y soporte para procesadores ARM de 32 bits (`armeabi-v7a`).

### 2. Contexto

- Contexto consultado: `android/AndroidManifest.xml`, `scripts/package_apk.ps1`, `src/packaging_tests.rs`, `AGENTS.md`.
- Intención: Resolver el error "No se instaló la aplicación" en televisores y TV boxes con Android TV (API 29 / Android 10), asegurando que el APK sea universal para dispositivos táctiles, emuladores y pantallas de televisión.
- Restricciones y decisiones previas (máx. 3):
  - La aplicación corre sobre `NativeActivity` con `hasCode="false"` (origen: `android/AndroidManifest.xml`) → implica: no depender de componentes Java Leanback (`LeanbackActivity`) ni librerías androidx-leanback en DEX; la declaración de compatibilidad de TV se realiza a nivel de manifiesto nativo. → protegida por: `Escenario: configuración de manifiesto para compatibilidad con Android TV`.
  - Portabilidad y privacidad estricta (origen: `AGENTS.md §6`) → implica: cero rutas locales absolutas en scripts, manifiestos o tests. → protegida por: `no testeable`.
  - Firma determinista multi-esquema (origen: compatibilidad con instaladores Android 7 a 15) → implica: apksigner debe firmar con esquemas v1, v2 y v3 simultáneamente. → protegida por: `Escenario: firma híbrida v1, v2 y v3 en arnés de empaquetado`.
- Alternativas descartadas:
  - Crear un APK separado exclusivamente para Android TV → descartada porque un Fat APK universal con `required="false"` en pantalla táctil y soporte multi-ABI simplifica la distribución y mantenimiento.
  - Obligar al usuario a instalar por comandos ADB con flags de bypass (`pm install -r -d`) → descartada porque los usuarios finales instalan por pendrive USB o sideloaders gráficos en sus televisores.
- Ante imprevistos, prioriza: máxima compatibilidad de instalación en dispositivos heredados sobre peso del APK.

### 3. Ejemplos

- Caso normal (instalación en Smart TV con Android TV sin panel táctil):
  - Entrada: El instalador de Android TV (`PackageInstaller`) analiza el APK en Android 10.
  - Antes: Android asume que `android.hardware.touchscreen` es obligatorio, no detecta pantalla táctil física en la tele y aborta la instalación con `INSTALL_FAILED_MISSING_FEATURE` ("No se instaló la aplicación").
  - Ahora: El manifiesto declara `android.hardware.touchscreen` con `required="false"`, `leanback` con `required="false"`, y categoría `LEANBACK_LAUNCHER`; la instalación se completa con éxito y la app aparece en la fila de juegos/apps del televisor.
- Caso límite (TV Box con SO Android compilado en 32 bits `armeabi-v7a` y firma JAR v1 exigida por el gestor de archivos):
  - Entrada: Se intenta instalar en un dispositivo con CPU/OS de 32 bits usando un explorador de archivos básico de TV.
  - Antes: El APK solo contenía `arm64-v8a` y `x86_64`, y carecía de firma JAR v1 (`Verified using v1 scheme: false`); falla con `INSTALL_FAILED_NO_MATCHING_ABIS` o firma rechazada.
  - Ahora: El APK incluye firma v1 (`JAR signing`) y el pipeline compila/empaqueta la variante `armeabi-v7a` (`lib/armeabi-v7a/libblazing_recomp.so`), instalándose limpiamente.

### 4. Pasos

1. Actualizar `android/AndroidManifest.xml`:
   - Declarar `<uses-feature android:name="android.hardware.touchscreen" android:required="false" />`.
   - Declarar `<uses-feature android:name="android.software.leanback" android:required="false" />`.
   - Añadir `<category android:name="android.intent.category.LEANBACK_LAUNCHER" />` en el `intent-filter` de la actividad.
   - Declarar `android:isGame="true"` en `<application>`.
2. Actualizar `scripts/package_apk.ps1`:
   - En la invocación de `apksigner`, añadir `--v1-signing-enabled true` junto a `--v2-signing-enabled true` y `--v3-signing-enabled true`.
   - Asegurar el soporte de compilación y empaquetado para `armeabi-v7a` (`armv7-linux-androideabi`).
3. Instalar el target `armv7-linux-androideabi` en Rustup (`rustup target add armv7-linux-androideabi`) para permitir la compilación 32-bit.
4. Actualizar y ampliar la suite de pruebas en `src/packaging_tests.rs` para verificar las directivas de Android TV, firma v1 y mapeo de arquitecturas.
5. Reempaquetar el APK y verificar con `apksigner verify --verbose` que v1, v2 y v3 estén en `true`.

### 5. Qué más podría verse afectado

- `android/AndroidManifest.xml` (características de hardware y categorías de launcher).
- `scripts/package_apk.ps1` (parámetros de firma y soporte de arquitecturas).
- `src/packaging_tests.rs` (nuevas aserciones de empaquetado).

### 6. Suposiciones

- La TV del usuario corre sobre una arquitectura ARM estándar (`arm64-v8a` o `armeabi-v7a`). → prototipar: no
  → si es falso: se añadirá soporte para arquitecturas MIPS o x86 según corresponda.
- El instalador del TV rechaza el paquete por falta de `touchscreen=false` o por firma v1 ausente. → prototipar: no
  → si es falso: se analizarán logs de `adb logcat` en el TV vía red (`adb connect <ip-tv>:5555`).
- El compilador de Rust compila `armv7-linux-androideabi` con las mismas dependencias de `gba-core` y `wgpu`. → prototipar: sí
  → si es falso: se ajustarán flags de feature para el target de 32 bits.
  → resultado del prototipo: [se completará tras aprobación] | no detecta: bugs en drivers OpenGL ES 3.0 específicos del GPU Mali/PowerVR del TV.

### 7. Alcance

- Incluye:
  - Declaración explícita de `touchscreen` y `leanback` no obligatorios en el manifiesto.
  - Categoría `LEANBACK_LAUNCHER` y bandera `isGame="true"`.
  - Habilitación de firma v1 (JAR signing) en `package_apk.ps1`.
  - Target `armv7-linux-androideabi` (`armeabi-v7a`) integrado en el empaquetado Fat APK.
  - Tests automatizados en `src/packaging_tests.rs`.
- No incluye:
  - Navegación D-Pad completa dentro de los menús nativos del juego (forma parte del hito 0.4.0 en el roadmap).
  - Banner gráfico personalizado para Leanback de 320x180 px (se mantiene icono por defecto del sistema temporalmente).

### 8. Criterios de aceptación (no se modifican sin aprobación)

Escenario: configuración de manifiesto para compatibilidad con Android TV
Caso de uso: instalación en Android TV sin pantalla táctil | Tipo: normal
  Dado el archivo `android/AndroidManifest.xml`
  Cuando se inspeccionan las características de hardware y categorías de la actividad
  Entonces declara `android.hardware.touchscreen` con `required="false"`, `android.software.leanback` con `required="false"`, y la categoría `android.intent.category.LEANBACK_LAUNCHER`.
Test: `src/packaging_tests.rs::test_manifest_declares_android_tv_compatibility`
Estado previo: [pendiente tras aprobación]

Escenario: firma híbrida v1, v2 y v3 en arnés de empaquetado
Caso de uso: firma de APK apta para instaladores heredados y sideloaders de TV | Tipo: normal
  Dado el script `scripts/package_apk.ps1`
  Cuando se analiza el comando de invocación de `apksigner`
  Entonces incluye explícitamente `--v1-signing-enabled true`, `--v2-signing-enabled true` y `--v3-signing-enabled true`.
Test: `src/packaging_tests.rs::test_packaging_script_enables_v1_v2_v3_signatures`
Estado previo: [pendiente tras aprobación]

Escenario: soporte de arquitectura 32 bits armeabi-v7a en empaquetador
Caso de uso: dispositivos con sistema operativo Android de 32 bits | Tipo: límite
  Dado el script `scripts/package_apk.ps1`
  Cuando se consulta el diccionario de mapeo de arquitecturas a ABIs
  Entonces contiene el mapeo entre `armv7-linux-androideabi` y `armeabi-v7a`.
Test: `src/packaging_tests.rs::test_packaging_script_supports_armeabi_v7a`
Estado previo: [pendiente tras aprobación]

Escenario: regresión en suite existente de controles y renderizado
Caso de uso: ejecución de la suite de pruebas del proyecto | Tipo: regresión
  Dado el proyecto con los 29 tests unitarios actuales
  Cuando se ejecuta `cargo test --lib`
  Entonces los 29 tests existentes continúan pasando en verde.
Test: `cargo test --lib`
Estado previo: pasa (regresión)

### 9. Cómo lo comprobaré

- configuración de manifiesto para compatibilidad con Android TV → herramienta o comando: `cargo test --lib test_manifest_declares_android_tv_compatibility`
  → pasa si: el test valida la presencia de `touchscreen=false`, `leanback=false` y `LEANBACK_LAUNCHER` → no detecta: restricciones de versión de Android anteriores a API 26.
- firma híbrida v1, v2 y v3 en arnés de empaquetado → herramienta o comando: `cargo test --lib test_packaging_script_enables_v1_v2_v3_signatures`
  → pasa si: el test valida que los flags de apksigner están presentes en el script → no detecta: fallos si Java keytool no está en el PATH.
- soporte de arquitectura 32 bits armeabi-v7a en empaquetador → herramienta o comando: `cargo test --lib test_packaging_script_supports_armeabi_v7a`
  → pasa si: el test valida el mapeo de `armv7-linux-androideabi` a `armeabi-v7a` → no detecta: problemas específicos del linker Clang en CPUs ARMv7 viejas sin NEON.
- regresión en suite existente de controles y renderizado → herramienta o comando: `cargo test --lib`
  → pasa si: 29 tests existentes pasan en verde → no detecta: errores en tiempo de ejecución en la TV real.
- verificación de firma del APK binario final → herramienta o comando: `apksigner verify --verbose target/blazing-recomp-debug.apk`
  → pasa si: muestra `Verified using v1 scheme: true`, `v2: true` y `v3: true` → no detecta: problemas de hardware en el dispositivo físico.
- instalación en Android TV real → revisión humana: el APK se transfiere al Android TV y la instalación se completa mostrando "Se instaló la aplicación".
  → no detecta: bugs de mapeo de mandos en el menú principal.

### 10. Terminado cuando

Se cumplen todos los criterios de aceptación, los tests pasan en verde y el APK generado reporta firmas v1, v2 y v3 válidas.

### 11. Parar si

2 intentos fallidos seguidos con el mismo criterio. Detenerse, explicar qué se probó y marcar la propuesta como "discrepada".

### Correcciones previas a presentar

- Ninguna.

---

## Resultado (lo completa el agente después de editar)

### Predicho vs. real

- configuración de manifiesto para compatibilidad con Android TV: coincide
  Resultado real: `test_manifest_declares_android_tv_compatibility` pasó en verde, validando `android.hardware.touchscreen` con `required="false"`, `leanback` con `required="false"`, `LEANBACK_LAUNCHER` e `isGame="true"`.
  Método: el anunciado (`cargo test --lib`).
- firma híbrida v1, v2 y v3 en arnés de empaquetado: coincide
  Resultado real: `test_packaging_script_enables_v1_v2_v3_signatures` pasó en verde y `apksigner verify --verbose --min-sdk-version 23` confirmó `Verified using v1 scheme: true`, `v2: true` y `v3: true`.
  Método: el anunciado (`cargo test --lib` y `apksigner verify`).
- soporte de arquitectura 32 bits armeabi-v7a en empaquetador: coincide
  Resultado real: `test_packaging_script_supports_armeabi_v7a` pasó en verde, el compilador NDK generó `target/armv7-linux-androideabi/debug/libblazing_recomp.so` y el APK final incluye las tres arquitecturas (`arm64-v8a`, `armeabi-v7a`, `x86_64`).
  Método: el anunciado (`cargo test --lib` y empaquetado con `package_apk.ps1`).
- regresión en suite existente de controles y renderizado: coincide
  Resultado real: Los 32 tests de la suite pasaron en verde.
  Método: el anunciado (`cargo test --lib`).

### Contexto: ¿se respetó?

- NativeActivity pura sin DEX Java ni librerías leanback Java → respetada: la compatibilidad de TV se logró a nivel de manifiesto declarativo sin alterar la naturaleza nativa del ejecutable.
- Portabilidad y privacidad estricta → respetada: cero rutas locales absolutas introducidas.
- Firma determinista multi-esquema → respetada: el APK contiene `META-INF/MANIFEST.MF`, `.SF`, `.RSA` y los bloques de firma v2 y v3.
- ¿Se tocó algo que alguna restricción prohibía? no.
- Imprevistos que surgieron: la aserción inicial en el test comprobaba atributos contiguos en la misma línea en lugar de validar las etiquetas XML independientemente del salto de línea; se ajustó la aserción en el test.
- ¿Reapareció alguna alternativa descartada? no.

### Archivos modificados

- `android/AndroidManifest.xml` (atributos de Android TV, pantalla táctil opcional y launcher Leanback)
- `scripts/package_apk.ps1` (soporte multi-ABI automático y activación de firma v1 en apksigner)
- `src/packaging_tests.rs` (nuevos tests de manifiesto, firma v1 y arquitectura 32-bit)
- ¿Coincide con lo anunciado? sí.

### Discrepancias

Ninguna.

### Para la próxima vez

Mantener por defecto el ensamblado Fat APK con las tres arquitecturas (`arm64-v8a`, `armeabi-v7a`, `x86_64`) para cubrir el 100% de dispositivos físicos (móviles y televisores) y emuladores.

### Fuera de alcance (hallazgos que no se arreglaron)

- Banner gráfico personalizado para Leanback de 320x180 px | impacto: bajo.

---

## Revisión humana

- Instalación en Android TV real (visible): ✔
- Portabilidad y privacidad estricta (no testeable): ✔
- Detectado solo por el humano: Instalación y arranque confirmados en Android TV 10, Tablet Android 11 y emuladores.

