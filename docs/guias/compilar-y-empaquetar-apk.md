---
tipo: guia-practica
verificado: si
---

# Cómo compilar, empaquetar e instalar el APK de Blazing Recomp

Esta guía práctica detalla los pasos para construir las bibliotecas nativas C11 y Rust, ensamblar el paquete instalador Fat APK multi-arquitectura y desplegarlo en un dispositivo Android físico o emulador.

---

## Requisitos

Antes de iniciar el empaquetado, asegúrate de contar con las siguientes herramientas en tu entorno de desarrollo:
1. **Rust Toolchain:** Versión estable con los targets de Android instalados:
   ```bash
   rustup target add aarch64-linux-android armv7-linux-androideabi x86_64-linux-android
   ```
2. **Cargo NDK:** Utilidad para compilar crates de Rust contra el Android NDK:
   ```bash
   cargo install cargo-ndk
   ```
3. **Android NDK y SDK Command-Line Tools:**
   - Variable de entorno `ANDROID_HOME` o `ANDROID_SDK_ROOT` apuntando al SDK de Android.
   - Variable de entorno `ANDROID_NDK_HOME` o subdirectorio `ndk/<version>` accesible en el SDK.
   - Herramientas `aapt2`, `zipalign` y `apksigner` en la variable de entorno `PATH` o dentro de `build-tools/<version>`.
4. **Dispositivo o Emulador Android:** Con depuración USB (*USB Debugging*) activada y visible mediante el comando `adb devices`.
5. **Repositorio Hermano del Motor Upstream (`../gba-recomp`):** Debido a que `Cargo.toml` utiliza la directiva `[patch."https://github.com/JRickey/gba-recomp.git"]` vinculando `../gba-recomp/crates/gba-core`, dicho repositorio debe estar clonado de forma adyacente (`git clone https://github.com/JRickey/gba-recomp.git ../gba-recomp`) para compilar con `cargo-ndk` y validar con `cargo test`.

---

## Pasos

### 1. Verificar la integridad de los componentes previos
Comprueba que los tests unitarios y la auditoría de etiquetas estén en verde antes de construir:
```bash
cargo test
python scripts/diagnostico/auditar_labels.py
```

### 2. Ejecutar el script oficial de empaquetado
Invoca el arnés de construcción automatizado de PowerShell desde la raíz del repositorio:
```powershell
powershell -ExecutionPolicy Bypass -File scripts/package_apk.ps1
```

El script ejecuta de forma desatendida las siguientes fases:
1. Compila las bibliotecas nativas `libblazing_recomp.so` (runtime en Rust) para `arm64-v8a`, `armeabi-v7a` y `x86_64` con `cargo-ndk` en modo de lanzamiento (`--release`).
2. Procesa el manifiesto `android/AndroidManifest.xml` y los recursos con `aapt2`.
3. Empaqueta el archivo ZIP intermedio incorporando en `lib/<abi>/` ambas bibliotecas dinámicas contiguas de la aplicación:
   - `libblazing_recomp.so`: Runtime en Rust y punto de entrada `NativeActivity`.
   - `libblazing_recompiled.so`: Lógica C11 recompilada AOT (los 144.875 bloques del juego compilados previamente con Clang del NDK a partir de `src-c/`).
4. Alinea las páginas de memoria a 16 KB (16,384 bytes) mediante `zipalign`.
5. Firma digitalmente el archivo con `apksigner` bajo los esquemas v1, v2 y v3.

### 3. Instalar el APK en el dispositivo Android
Conecta tu smartphone, tablet o Android TV por USB o red local (ADB over Wi-Fi), y ejecuta:
```bash
adb install -r target/blazing-recomp-release.apk
```
*Si estás instalando en un dispositivo Android TV o TV Stick, el sistema lo reconocerá automáticamente gracias a la declaración `LEANBACK_LAUNCHER` en el manifiesto.*

### 4. Aprovisionar la ROM y BIOS oficial en el dispositivo
Por razones legales y de seguridad, el APK no incluye assets propietarios del juego. La app busca los archivos oficiales verificando sus hashes criptográficos en el almacenamiento del dispositivo:
- **Directorio externo privado de la app (ruta recomendada, no requiere permisos):**
  `/sdcard/Android/data/com.blazingrecomp.game/files/`
- **Almacenamiento compartido:**
  `/sdcard/Download/` (o `/storage/emulated/0/Download/`)

Copia los archivos legales desde tu ordenador mediante `adb`:
```bash
adb push "Fire Emblem (Europe).gba" /sdcard/Android/data/com.blazingrecomp.game/files/
adb push gba_bios.bin /sdcard/Android/data/com.blazingrecomp.game/files/
```
*(También puedes enviarlos a `/sdcard/Download/` si utilizas un explorador de archivos en el teléfono).*

---

## Comprobar el resultado

Para confirmar que la compilación, firma, instalación y arranque fueron exitosos:

1. **Verificar la firma del paquete generado:**
   ```bash
   apksigner verify --verbose target/blazing-recomp-release.apk
   ```
   Debes observar la confirmación de esquemas activos:
   ```text
   Verifies
   Verified using v1 scheme (JAR signing): true
   Verified using v2 scheme (APK Signature Scheme v2): true
   Verified using v3 scheme (APK Signature Scheme v3): true
   ```

2. **Verificar la alineación de memoria a 16 KB:**
   ```bash
   zipalign -c -v 16 target/blazing-recomp-release.apk
   ```
   El verificador debe finalizar con `Verification successful`.

3. **Ejecutar la aplicación en el dispositivo:**
   Inicia la aplicación en Android desde el lanzador o con `adb shell monkey -p com.blazingrecomp.game 1`. Si la ROM o la BIOS aún no se han copiado, la aplicación mostrará una pantalla de diagnóstico en el framebuffer indicando los hashes requeridos y las rutas escaneadas. Una vez detectados los archivos con sus hashes SHA-256 correctos, el juego iniciará fluidamente a 60 FPS con el overlay táctil en pantalla.
