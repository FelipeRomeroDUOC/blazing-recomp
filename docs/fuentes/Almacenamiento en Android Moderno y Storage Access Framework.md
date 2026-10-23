# Almacenamiento en Android Moderno y Storage Access Framework (SAF)

## 1. Evolución del Modelo de Seguridad: Del Sistema de Archivos POSIX a *Scoped Storage*

En los inicios del sistema operativo Android (versiones 1.0 a 9.0 / APIs 1 a 28), las aplicaciones nativas en C/C++ y Rust podían interactuar con el almacenamiento del dispositivo empleando las llamadas convencionales de la biblioteca estándar de C y del kernel de Linux (`open()`, `read()`, `write()`, `mmap()`) sobre rutas absolutas del sistema de archivos (`/sdcard/`, `/storage/emulated/0/`). Para acceder a archivos compartidos (como imágenes de ROMs o carpetas de descargas), bastaba con declarar los permisos `READ_EXTERNAL_STORAGE` y `WRITE_EXTERNAL_STORAGE` en el manifiesto (`AndroidManifest.xml`) y solicitarlos dinámicamente en tiempo de ejecución.

A partir de **Android 10 (API 29)** y consolidado de forma obligatoria en **Android 11 (API 30)**, Google introdujo una reestructuración profunda conocida como **Almacenamiento con Alcance (*Scoped Storage*)**. Este modelo impone un estricto aislamiento de datos (*sandboxing*) para proteger la privacidad de los usuarios:

1. **Aislamiento del Almacenamiento Externo:** Las aplicaciones ya no tienen visibilidad del almacenamiento compartido mediante rutas POSIX del sistema de archivos. El intento de acceder a rutas como `/sdcard/Download/game.gba` mediante funciones nativas de C/Rust falla inmediatamente con un error de acceso denegado (`EACCES` / `errno 13`), independientemente de los permisos declarados en el manifiesto.
2. **Carpetas Específicas de la Aplicación (*App-Specific Directories*):** Cada aplicación conserva acceso POSIX directo y transparente únicamente dentro de su propio directorio privado en el almacenamiento interno (`/data/data/<package>/files/`) y externo (`/sdcard/Android/data/<package>/files/`). Sin embargo, los datos alojados en estas rutas son eliminados irrevocablemente por el sistema si el usuario desinstala la aplicación, haciéndolas inadecuadas para partidas guardadas de largo plazo o bibliotecas de ROMs que el usuario desea conservar.
3. **El Storage Access Framework (SAF) como Pasarela Obligatoria:** Para acceder a archivos o directorios ubicados fuera del sandbox privado, las aplicaciones deben utilizar el marco de trabajo **Storage Access Framework (SAF)**, donde el usuario selecciona explícitamente qué archivo o carpeta autoriza mediante el selector de documentos del sistema operativo (*System Document Picker*).

```
+-------------------------------------------------------------------------------+
|                    ARQUITECTURA DE ALMACENAMIENTO EN ANDROID                  |
|                                                                               |
|  [Rutas POSIX Tradicionales] (/sdcard/...)                                    |
|   - Android <= 9.0: Acceso directo permitido con permisos de lectura.        |
|   - Android >= 10.0: BLOQUEADO (EACCES / Permission Denied).                  |
|                                                                               |
|  [Sandbox Privado] (/data/data/<pkg>/ o /sdcard/Android/data/<pkg>/)          |
|   - Acceso POSIX directo sin permisos. Se destruye al desinstalar.            |
|                                                                               |
|  [Storage Access Framework (SAF)]                                             |
|   - Usuario selecciona archivo/carpeta mediante System Document Picker.       |
|   - Devuelve una URI virtual (content://...).                                 |
|   - Se debe persistir el permiso (takePersistableUriPermission).              |
|   - Se extrae un Descriptor de Archivo POSIX nativo (ParcelFileDescriptor).   |
|   - El motor en C/Rust opera a máxima velocidad nativa sobre el fd.           |
+-------------------------------------------------------------------------------+
```

---

## 2. El Storage Access Framework (SAF) y la Semántica de URIs de Contenido

El Storage Access Framework desacopla la ubicación física del archivo de la aplicación que lo consume. En lugar de exponer rutas de ruta de archivo en disco (como `/storage/emulated/0/GBA/fe7.gba`), el sistema emite una **URI de Contenido (*Content URI*)** estructurada de forma opaca:

```text
content://com.android.externalstorage.documents/document/primary%3AGBA%2Ffe7.gba
```

### Componentes Arquitectónicos del SAF
- **Proveedor de Documentos (*DocumentProvider*):** Servicio del sistema que expone el acceso a los nodos de almacenamiento (almacenamiento flash interno, tarjetas MicroSD externas o servicios de nube como Google Drive).
- **Selector de Documentos del Sistema (*System Document Picker*):** Actividad del sistema (`Intent.ACTION_OPEN_DOCUMENT` o `ACTION_OPEN_DOCUMENT_TREE`) ejecutada en un proceso independiente fuera de la aplicación. Garantiza que la aplicación no pueda explorar el disco en busca de archivos sin intervención del usuario.
- **Cliente (*Client Application*):** La aplicación que registra el selector y recibe el resultado mediante el callback de actividad (`onActivityResult` / `ActivityResultLauncher`).

### Retención de Permisos: El Desafío de la Persistencia
Por defecto, las credenciales de lectura y escritura asociadas a una URI devuelta por el selector de documentos son **temporales y efímeras**:
- El permiso caduca inmediatamente cuando el proceso de la aplicación finaliza o cuando el dispositivo se reinicia.
- Si una aplicación almacena la cadena URI `content://...` en una base de datos o archivo de configuración e intenta reabrirla en el siguiente arranque, la llamada fallará con una excepción de seguridad (`SecurityException: Permission Denial`).

Para garantizar que un emulador o puerto nativo retenga acceso continuo a las ROMs y partidas guardadas entre sesiones, la aplicación Java/Kotlin debe consolidar el permiso invocando explícitamente el método `takePersistableUriPermission` del `ContentResolver`:

```java
int takeFlags = intent.getFlags() & 
    (Intent.FLAG_GRANT_READ_URI_PERMISSION | Intent.FLAG_GRANT_WRITE_URI_PERMISSION);

// Consolida el permiso de forma indefinida en la base de datos de seguridad del sistema
context.getContentResolver().takePersistableUriPermission(selectedUri, takeFlags);
```

---

## 3. El Puente Nativo: Descriptores de Archivos POSIX (`int fd`) hacia Rust y C

Un error frecuente de diseño consiste en intentar adaptar motores nativos leyendo las URIs del SAF a través de la biblioteca auxiliar de Java `androidx.documentfile.provider.DocumentFile`. 

### La Ineficiencia de `DocumentFile`
Cada consulta realizada a través de `DocumentFile` (como `exists()`, `length()`, `canWrite()` o `openInputStream()`) se traduce internamente en una llamada a través de la base de datos `ContentProvider` mediante una transacción de Comunicación Inter-Proceso (*IPC via Binder*).
- En operaciones masivas o accesos repetidos, esta sobrecarga consume entre **10 ms y 50 ms por cada acceso a archivo**, lo que arruina el rendimiento y la capacidad de respuesta en un juego de 60 FPS.

### La Solución de Alto Rendimiento: Extracción del Descriptor de Archivo (`fd`)
El kernel de Linux y la arquitectura de Android proporcionan un mecanismo de acceso directo de cero copias (*zero-copy*): el descriptor de archivo subyacente. A través de la clase `ParcelFileDescriptor`, la capa Java puede abrir el canal de E/S del sistema de archivos real y transferir el identificador numérico entero (`fd`) directamente a la biblioteca nativa en Rust o C:

```java
// 1. Apertura del descriptor en modo lectura ("r") o lectura/escritura ("rw" o "rwt")
ParcelFileDescriptor pfd = context.getContentResolver().openFileDescriptor(uri, "rw");

// 2. Extracción del descriptor entero de archivo POSIX
int rawFd = pfd.detachFd(); // detachFd() cede la propiedad del fd al runtime nativo
```

### Recepción y Uso en Rust
Una vez transferido el entero `rawFd` a través de JNI, el motor en Rust puede reconstruir estructuras nativas del sistema operativo sin incurrir en ninguna sobrecarga de Java ni de Binder:

```rust
use std::fs::File;
use std::os::unix::io::{FromRawFd, RawFd};

pub fn inicializar_rom_desde_saf(raw_fd: RawFd) -> Result<File, std::io::Error> {
    // Convierte el descriptor nativo en un objeto File estándar de Rust.
    // La estructura File asume la propiedad del descriptor y lo cerrará al salir de ámbito.
    let file = unsafe { File::from_raw_fd(raw_fd) };
    Ok(file)
}
```

A partir de este instante, el código nativo interactúa con el archivo empleando las APIs estándar de POSIX a velocidad directa del kernel de Linux, con tiempos de respuesta en el orden de los sub-microsegundos.

---

## 4. Mapeo Directo en Memoria (*Memory Mapping / `mmap`*) sobre Descriptores de SAF

Para imágenes de ROM de Game Boy Advance (archivos estáticos de 16 MB o 32 MB en cartucho), la técnica más eficiente en consumo de memoria RAM y rendimiento es el mapeo directo en el espacio virtual de direcciones del proceso mediante la llamada al sistema `mmap()`.

### Ventajas de `mmap` sobre la Carga Completa en Heap
1. **Cero Copias en RAM Dinámica:** Al invocar `mmap()`, no se asignan 16 MB contiguos en el montículo (*heap*) de la aplicación. En su lugar, el kernel de Linux asocia el rango de direcciones virtuales del proceso directamente a los bloques físicos en el almacenamiento flash.
2. **Paginación por Demanda (*Demand Paging*):** Las secciones de la ROM se cargan en la memoria RAM física únicamente cuando la CPU intenta leerlas por primera vez. Si ciertas subrutinas o escenas de *Fire Emblem* no se ejecutan durante la partida, sus datos nunca consumen memoria RAM.
3. **Liberación Automática de Memoria por el Kernel:** En dispositivos con poca memoria RAM disponible (como TV Sticks con 1 GB o 1.5 GB de RAM total), si el sistema sufre presión de memoria, el kernel puede descartar transparentemente páginas de solo lectura de la ROM que no hayan sido modificadas, recuperándolas del almacenamiento flash si vuelven a ser requeridas sin necesidad de cerrar la aplicación.

```rust
use memmap2::MmapOptions;
use std::fs::File;

pub fn mapear_rom(file: &File) -> Result<memmap2::Mmap, std::io::Error> {
    // Mapea la ROM en memoria virtual en modo de solo lectura compartido
    let mmap = unsafe { MmapOptions::new().map(file)? };
    Ok(mmap)
}
```

---

## 5. Persistencia Determinista de Partidas Guardadas (SRAM / Flash)

El almacenamiento no volátil en la Game Boy Advance (típicamente SRAM de 32 KB o 64 KB, Flash de 64 KB o 128 KB, o EEPROM) requiere un tratamiento diferenciado del de las ROMs debido a su carácter de lectura y escritura bidireccional.

### El Peligro del Búfer Sucio en Almacenamiento Flash
Cuando un juego como *Fire Emblem* actualiza la posición de las unidades o registra el final de un turno en el mapa, escribe datos en el espacio de la batería (`0x0E000000`). Si el motor nativo escribe en el archivo mediante llamadas `write()` convencionales de Linux, el sistema operativo retiene los datos en la memoria caché de páginas (*Page Cache*) de la RAM para optimizar el rendimiento y reducir el desgaste del silicio flash.
- Si el usuario conmuta a otra aplicación en Android y el sistema operativo finaliza el proceso en segundo plano por falta de memoria (*Low Memory Killer, LMK*), o si el dispositivo sufre un corte de energía, los datos retenidos en la caché de páginas se pierden definitivamente, provocando la corrupción o pérdida de la partida guardada.

### Garantía de Integridad mediante Sincronización Forzada
Para lograr una persistencia infalible al guardar datos mediante el descriptor de archivo de SAF:
1. **Modo de Apertura del Descriptor:** El descriptor `pfd` debe solicitarse obligatoriamente con el modo `"rw"` o `"rwt"` (donde `"t"` trunca el archivo si se requiere reemplazarlo por completo).
2. **Sincronización Física con el Silicio (`fsync`):** Inmediatamente después de completar la escritura del búfer de la partida guardada, el motor nativo en Rust/C debe invocar la llamada al sistema `fsync()` (o el método `sync_all()` / `sync_data()` de Rust) sobre el descriptor de archivo:
   ```rust
   use std::fs::File;
   use std::io::Write;

   pub fn guardar_sram(file: &mut File, buffer_sram: &[u8]) -> Result<(), std::io::Error> {
       file.rewind()?;
       file.write_all(buffer_sram)?;
       // Fuerza el vaciado de los bloques de la caché del kernel hacia el almacenamiento físico
       file.sync_data()?;
       Ok(())
   }
   ```
   Esta llamada bloquea la operación hasta que el controlador de la memoria flash eMMC o UFS confirma que los bloques han sido escritos físicamente en las celdas de almacenamiento no volátil, garantizando que la partida guardada sobreviva a cierres abruptos o reinicios del sistema.
