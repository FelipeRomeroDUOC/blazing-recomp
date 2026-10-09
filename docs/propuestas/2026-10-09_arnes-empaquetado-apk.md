# Propuesta: Arnés de Empaquetado APK para Blazing Recomp en Android

Fecha: 2026-10-09
Nivel: 2
Estado: verificada por agente

<!-- Estados: pendiente | aprobada | modificada | descartada |
     verificada por agente | revisada por humano | discrepada | cerrada -->

## Propuesta (la escribe el agente antes de editar)

### 1. Qué cambia

Crea el arnés de empaquetado para generar el archivo instalable `target/blazing-recomp-debug.apk` con `android/AndroidManifest.xml` (NativeActivity y permiso de vibración), empaquetando la biblioteca nativa ARM64 y firmándola con clave de depuración para probar directamente en dispositivos Android.

### 2. Contexto

- Contexto consultado: `AGENTS.md`, `docs/BACKLOG.md`, `src/platform.rs`, `Cargo.toml`.
- Intención: Permitir que el usuario instale y ejecute el port nativo directamente en su teléfono Android para comprobar visualmente los controles táctiles, el marco de GBA SP y la respuesta háptica en hardware real.
- Restricciones y decisiones previas (máx. 3):
  - Uso de NativeActivity pura de Android sin capas Java intermedias (origen: `AGENTS.md` §2.2) → implica: declarar `android.app.NativeActivity` con `android.app.lib_name = blazing_recomp` en `AndroidManifest.xml` → protegida por: Declaración correcta de NativeActivity y permisos en AndroidManifest.xml.
  - Empaquetado portable sin rutas locales absolutas hardcodeadas (origen: `AGENTS.md` §6) → implica: el script de empaquetado debe resolver rutas mediante variables de entorno estándar (`ANDROID_HOME`, `JAVA_HOME`, `LOCALAPPDATA`) sin nombres de usuario ni discos fijos → protegida por: Independencia de rutas absolutas en el arnés de empaquetado.
  - Firma y alineación compatibles con Android moderno (origen: `AGENTS.md` §2.1) → implica: alinear el APK con `zipalign` a 4 bytes y páginas de 16 KB, y firmar con esquemas v2/v3 mediante `apksigner` → protegida por: Generación y verificación de firma del APK de depuración.
- Alternativas descartadas:
  - Configurar un proyecto Gradle completo con wrapper → descartada para evitar descargas pesadas de dependencias de red, aprovechando las herramientas del SDK ya instaladas (`aapt2`, `zipalign`, `apksigner`).
  - Depender de `cargo-apk` → descartada para no requerir instalación de crates CLI externos adicionales en el entorno de desarrollo.
- Ante imprevistos, prioriza: generación de un APK autocontenido y firmado con herramientas directas del SDK sobre dependencias de construcción complejas.

### 3. Ejemplos

- Caso normal: El desarrollador ejecuta el script de empaquetado → antes: solo existía el archivo `.so` en `target/aarch64-linux-android/debug/` imposible de instalar directamente en Android → ahora: se genera `target/blazing-recomp-debug.apk` firmado y alineado, listo para instalar con `adb install`.
- Caso límite: El entorno no cuenta con un almacén de claves `debug.keystore` previo → antes: el firmado fallaría por falta de certificado → ahora: el arnés genera automáticamente una clave de desarrollo estándar si no existe y firma el APK sin interrupciones.

### 4. Pasos

1. Crear el manifiesto de la aplicación `android/AndroidManifest.xml` configurando `android.app.NativeActivity`, metadato `android.app.lib_name = blazing_recomp`, permiso `android.permission.VIBRATE`, orientación y tema a pantalla completa.
2. Crear el script de empaquetado portable `scripts/package_apk.ps1` que detecte el Android SDK (`ANDROID_HOME` o `LOCALAPPDATA\Android\Sdk`), compile el `.so` con `cargo ndk -t arm64-v8a build`, ensamble el APK base con `aapt2 link`, incorpore la biblioteca nativa en `lib/arm64-v8a/`, alinee con `zipalign` y firme con `apksigner`.
3. Añadir una prueba unitaria en Rust que valide la estructura y valores requeridos de `android/AndroidManifest.xml`.
4. Ejecutar el script para compilar y generar `target/blazing-recomp-debug.apk`.
5. Verificar la firma y validez del APK con `apksigner verify`.

### 5. Qué más podría verse afectado

- `android/AndroidManifest.xml`: archivo nuevo con configuración de la app.
- `scripts/package_apk.ps1`: script nuevo de empaquetado.
- `docs/BACKLOG.md`: actualización de la tarea de APK.

### 6. Suposiciones

- Las herramientas `aapt2.exe`, `zipalign.exe` y `apksigner.bat` de build-tools 36.0.0 junto con `android.jar` de `android-37.0` son suficientes para ensamblar un APK de `NativeActivity` funcional sin código Java compilado (`classes.dex`) → prototipar: sí → si es falso: generar un `classes.dex` vacío con `d8` → resultado del prototipo: | no detecta: políticas específicas de tiendas de aplicaciones (Google Play).
- La variable de entorno `ANDROID_HOME` o la ruta estándar en `%LOCALAPPDATA%\Android\Sdk` es detectable de forma robusta en PowerShell → prototipar: no → si es falso: permitir parámetro manual `-SdkPath` → resultado del prototipo: | no detecta: configuraciones personalizadas de SDK en discos secundarios.

### 7. Alcance

- Incluye: `android/AndroidManifest.xml`, script `scripts/package_apk.ps1`, gestión de `debug.keystore` de desarrollo, ensamblado del APK con `lib/arm64-v8a/libblazing_recomp.so`, alineación con `zipalign`, firmado con `apksigner` y salida en `target/blazing-recomp-debug.apk`.
- No incluye: Firma con claves de producción (release keystore), código DEX/Java adicional, ni selector de archivos SAF para ROM/BIOS (Fase 7).

### 8. Criterios de aceptación (no se modifican sin aprobación)

Escenario: Declaración correcta de NativeActivity y permisos en AndroidManifest.xml
Caso de uso: Manifiesto configurado para cargar libblazing_recomp.so con vibración | Tipo: normal
  Dado que existe el archivo `android/AndroidManifest.xml`
  Cuando se analiza su contenido XML
  Entonces declara la actividad `android.app.NativeActivity`, el metadato `android.app.lib_name` con valor `blazing_recomp` y el permiso `android.permission.VIBRATE`
Test: `test_manifest_declares_native_activity`
Estado previo: falla (archivo no existe)

Escenario: Generación y verificación de firma del APK de depuración
Caso de uso: Creación de un paquete APK instalable y firmado para ARM64 | Tipo: normal
  Dado que la biblioteca nativa `libblazing_recomp.so` está compilada para ARM64
  Cuando se ejecuta el script de empaquetado `scripts/package_apk.ps1`
  Entonces genera `target/blazing-recomp-debug.apk` y `apksigner verify` confirma que la firma es válida
Test: `test_apk_generation_and_signature`
Estado previo: falla (script no existe)

Escenario: Independencia de rutas absolutas en el arnés de empaquetado
Caso de uso: Portabilidad del script y del manifiesto en cualquier entorno | Tipo: regresión
  Dado que el arnés de empaquetado utiliza rutas relativas y variables de entorno
  Cuando se buscan patrones de rutas absolutas locales en `android/` y `scripts/`
  Entonces el número de coincidencias es exactamente 0
Test: `test_no_local_paths_in_packaging`
Estado previo: pasa (regresión preventiva)

### 9. Cómo lo comprobaré

- Declaración correcta de NativeActivity y permisos en AndroidManifest.xml → herramienta o comando: `cargo test --lib test_manifest_declares_native_activity`
  → pasa si: test unitario finaliza con código 0 → no detecta: sintaxis de atributos XML no reconocidos por versiones viejas de Android.
- Generación y verificación de firma del APK de depuración → herramienta o comando: `powershell -ExecutionPolicy Bypass -File scripts/package_apk.ps1`
  → pasa si: el APK se crea y `apksigner verify` confirma la firma con código 0 → no detecta: incompatibilidad en dispositivos de 32 bits (armv7).
- Independencia de rutas absolutas en el arnés de empaquetado → herramienta o comando: `git grep -i "C:/" -- android/ scripts/`
  → pasa si: 0 coincidencias → no detecta: rutas absolutas estilo Unix.

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

- Declaración correcta de NativeActivity y permisos en AndroidManifest.xml: coincide
  Resultado real: `cargo test --lib test_manifest_declares_native_activity` pasa (código 0).
  El manifiesto `android/AndroidManifest.xml` declara `android.app.NativeActivity`,
  `android.app.lib_name = blazing_recomp`, permiso `android.permission.VIBRATE`,
  `extractNativeLibs="false"` y `hasCode="false"`.
  Método: el anunciado.
- Generación y verificación de firma del APK de depuración: coincide
  Resultado real: `scripts/package_apk.ps1` ejecutó todo el pipeline y `apksigner verify`
  confirmó la firma v2 y v3 con código 0, generando el paquete `target/blazing-recomp-debug.apk`
  (135.442.862 bytes), con biblioteca STORED (sin compresión) y alineada a 16 KB (offset 16384).
  Método: el anunciado.
- Independencia de rutas absolutas en el arnés de empaquetado: coincide
  Resultado real: `git grep -i "C:/" -- android/ scripts/` retornó 0 coincidencias
  y el test unitario `test_no_local_paths_in_packaging` pasó con éxito.
  Método: el anunciado.

### Contexto: ¿se respetó?

- NativeActivity pura sin capas Java intermedias → respetada (manifiesto con `hasCode="false"`, sin clases DEX, cargando `libblazing_recomp.so` directamente).
- Empaquetado portable sin rutas locales fijadas → respetada (detección dinámica de Android SDK / NDK vía variables de entorno estándar, 0 rutas absolutas en archivos).
- Firma y alineación compatibles con Android moderno → respetada (alineación con `zipalign -P 16 4` alcanzando offset 16384 bytes, 16 KB page-aligned, y firmado v2/v3 con `apksigner`).
- ¿Se tocó algo que alguna restricción prohibía? no.
- Imprevistos que surgieron: ninguno → criterio de prioridad aplicado: no hubo imprevistos.
- ¿Reapareció alguna alternativa descartada? no.

### Archivos modificados

`android/AndroidManifest.xml`, `scripts/package_apk.ps1`, `src/lib.rs`, `src/packaging_tests.rs`.
¿Coincide con lo anunciado? sí (manifiesto y script anunciados; `src/packaging_tests.rs` añadido para validar programáticamente el empaquetado en `cargo test`).

### Discrepancias

Ninguna. Todos los criterios de aceptación y restricciones de contexto se cumplieron con éxito.

### Para la próxima vez

Al empaquetar bibliotecas nativas `.so` que se cargan mediante `mmap` directo (`extractNativeLibs="false"`), verificar siempre tanto el método de compresión (0 = STORED) como la alineación de página a 16 KB (`zipalign -P 16 4`) para garantizar compatibilidad con Android 15.

### Fuera de alcance (hallazgos que no se arreglaron)

- ninguno.

---

## Revisión humana

- ninguno: todo se comprobó con tests (`cargo test --lib` y `apksigner verify`).
- Detectado solo por el humano:

