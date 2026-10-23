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
1. Compila las bibliotecas nativas `libblazing_recomp.so` para `arm64-v8a`, `armeabi-v7a` y `x86_64` con optimizaciones de lanzamiento (`--release`).
2. Procesa el manifiesto `android/AndroidManifest.xml` y los recursos con `aapt2`.
3. Empaqueta el archivo ZIP intermedio incorporando las bibliotecas nativas en `lib/<abi>/`.
4. Alinea las páginas de memoria a 16 KB (16,384 bytes) mediante `zipalign`.
5. Firma digitalmente el archivo con `apksigner` bajo los esquemas v1, v2 y v3.

### 3. Instalar el APK en el dispositivo Android
Conecta tu smartphone, tablet o Android TV por USB o red local (ADB over Wi-Fi), y ejecuta:
```bash
adb install -r android/build/blazing-recomp.apk
```
*Si estás instalando en un dispositivo Android TV o TV Stick, el sistema lo reconocerá automáticamente gracias a la declaración `LEANBACK_LAUNCHER` en el manifiesto.*

---

## Comprobar el resultado

Para confirmar que la compilación, firma e instalación fueron exitosas:

1. **Verificar la firma del paquete generado:**
   ```bash
   apksigner verify --verbose android/build/blazing-recomp.apk
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
   zipalign -c -v 16 android/build/blazing-recomp.apk
   ```
   El verificador debe finalizar con `Verification successful`.

3. **Ejecutar la aplicación en el dispositivo:**
   Inicia la aplicación en Android. En el primer arranque, la aplicación mostrará la pantalla de selección o carga de ROM/BIOS. Al suministrar los archivos legales de *Fire Emblem: The Blazing Blade*, el juego debe iniciar fluidamente a 60 FPS.
