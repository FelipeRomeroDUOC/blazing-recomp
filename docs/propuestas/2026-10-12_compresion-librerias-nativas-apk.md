# Propuesta: Compresión de las librerías nativas del APK

Fecha: 2026-10-12
Nivel: 2
Estado: verificada por agente

<!-- Estados: pendiente | aprobada | modificada | descartada |
     verificada por agente | revisada por humano | discrepada | cerrada -->

## Propuesta (la escribe el agente antes de editar)

### 1. Qué cambia

El archivo instalable (`APK`) deja de guardar las seis bibliotecas nativas sin comprimir.
Hoy el APK pesa **195,670,857 bytes** (186.6 MiB) y esas seis entradas ocupan dentro del
archivo exactamente lo mismo que ocuparían fuera de él. Después del cambio, quien instala
el juego descarga aproximadamente una cuarta parte de ese archivo.

No cambia nada de lo que se ve al jugar: el manifiesto sigue declarando
`android:extractNativeLibs="true"`, así que Android sigue **extrayendo** las bibliotecas
al instalar, exactamente igual que hoy. Tampoco se toca el código recompilado ni su nivel
de optimización.

El cambio no modifica la lógica del emulador; solo cómo se guardan los archivos dentro
del ZIP del APK.

### 2. Ejemplos

- Caso normal: se empaqueta el APK con los tres ABIs → antes: las 6 entradas `lib/**/*.so`
  tienen `compression_method=0` y `comp_size == uncomp_size`, y el APK pesa 186.6 MiB →
  ahora: las 6 tienen `compression_method=8` con `comp_size < uncomp_size`, y el APK pesa
  60 MiB o menos.
- Caso límite: se empaqueta con `-SkipBuild -AllAbis:$false` → antes: el APK incluye los
  tres ABIs si sus `.so` siguen presentes en `target/` → ahora: incluye exactamente un ABI.

### 3. Pasos

1. En [`scripts/package_apk.ps1`](../../scripts/package_apk.ps1), quitar la opción `-0`
   de la llamada a `jar` (línea 393) para que `jar` comprima las entradas que inyecta.
   Solo afecta a lo que se le pasa con `-C $stagingDir "lib"`, es decir, `lib/`.
2. En el respaldo con Python del mismo paso (línea 400), cambiar
   `zipfile.ZIP_STORED` por `zipfile.ZIP_DEFLATED`.
3. Reescribir el comentario de las líneas 328-330 y el mensaje de la línea 331, que hoy
   afirman que el empaquetado sin compresión es imprescindible. Ese comentario justifica
   la decisión con `extractNativeLibs=false`, pero
   [`android/AndroidManifest.xml`](../../android/AndroidManifest.xml) declara `"true"`.
4. Envolver el bucle de ABIs adicionales (línea 371) en `if ($AllAbis)`, para que
   `-AllAbis:$false` produzca de verdad un APK de un solo ABI.
5. Actualizar los tests de `tests/packaging_tests.rs` que fijan el contrato antiguo y
   añadir los dos nuevos.
6. Actualizar la documentación afectada (se detalla en el paso 4 de "Qué más podría
   verse afectado").

### 4. Qué más podría verse afectado

Búsquedas hechas: `grep -rn "extractNativeLibs" android/ tests/` (3 coincidencias),
`grep -rn "STORED|package_apk" docs/ scripts/` (10 coincidencias relevantes),
`grep -rn "max-page-size|link-arg" .cargo/ Cargo.toml build.rs android/ scripts/`
(ninguna coincidencia).

- **`resources.arsc`:** hoy está guardado sin comprimir (`method=0`, medido), y Android 11
  o superior exige eso. `jar -u -f <apk> -C <staging> lib` solo reemplaza entradas bajo
  `lib/`, por lo que no lo toca. Evidencia a favor: en el APK actual el manifiesto está
  comprimido (`method=8`) mientras `resources.arsc` está sin comprimir (`method=0`), o sea
  que la mezcla de métodos ya convive en este archivo.
- **`tests/packaging_tests.rs`:** el test `test_apk_generation_and_signature` exige hoy
  `compression_method == 0` (línea 169) y alineación del inicio de datos a 16 KB
  (línea 179). Ese contrato corresponde al modo `extractNativeLibs="false"`, que este
  proyecto no usa.
- **Documentación:** `docs/explicacion/arquitectura-port-android.md` (§2.6, punto 4) y
  `docs/explicacion/DECISIONES.md` (ADR 14 y 15) describen el empaquetado sin compresión.
  `docs/referencia/CONFIGURACION_PACK.md` es específico de `pack.toml` y no menciona estos
  pasos; no se toca.
- **Sin efecto:** `src/`, `src-c/`, `Cargo.toml` y el código recompilado. No hay ningún
  archivo que dependa de cómo se comprime el APK.

### 5. Suposiciones

- Supongo que `jar -u -f <apk> -C <staging> lib` solo reemplaza las entradas bajo `lib/` y
  conserva el método de compresión de las demás. → si es falso: `resources.arsc` podría
  quedar comprimido y Android 11 o superior rechazaría la instalación. Lo detecta el
  criterio 2; el plan alternativo sería inyectar con Python abriendo el APK y reescribiendo
  únicamente `lib/`.
- Supongo que el umbral de 60 MiB es alcanzable. → medido el turno anterior recomprimiendo
  estas seis entradas reales del APK con deflate: **43.9 MB**. → si el resultado supera
  60 MiB: el APK igualmente baja de 186 MiB y reviso el umbral contigo antes de continuar.
- Supongo que el peso final es aproximadamente la suma de las entradas comprimidas más una
  sobrecarga pequeña de firma y directorio. → medido: sobrecarga actual = 195,670,857 −
  195,565,824 = **105,033 bytes** (0.1 MiB).
- Supongo que Android acepta instalar un APK con las bibliotecas comprimidas cuando
  `extractNativeLibs="true"`. → **no verificado**: no hay ningún dispositivo conectado por
  `adb` en este equipo, así que no puedo ejecutarlo yo. Solo se comprueba instalando.

### 6. Alcance

- Incluye: el cambio de compresión en `scripts/package_apk.ps1` (llamada a `jar`, respaldo
  con Python y comentarios); la guarda `-AllAbis`; los tests de empaquetado que fijan el
  contrato antiguo más dos nuevos; la documentación afectada.
- No incluye: cambiar `extractNativeLibs` a `"false"`; reducir el peso del código C11
  recompilado (~62 MB por ABI); bajar el nivel de optimización del AOT; el App Bundle de
  Google Play (haría los recortes por dispositivo automáticamente, pero implica salir del
  empaquetado propio); documentar el resto de opciones del script, que hoy no están
  documentadas en ninguna parte.

### 7. Criterios de aceptación (no se modifican sin aprobación)

Escenario 1: **El APK de tres ABIs guarda las librerías comprimidas y pesa 60 MiB o menos**
  Dado el APK generado por `scripts/package_apk.ps1` con los tres ABIs
  Cuando se leen las entradas cuyo nombre empieza por `lib/` y termina en `.so`
  Entonces hay exactamente 6, todas con `compression_method == 8` y `comp_size < uncomp_size`,
    y el tamaño total del archivo es menor o igual a 62,914,560 bytes
Test: `tests/packaging_tests.rs::test_apk_generation_and_signature` (reescrito)
Estado previo: **falla** (hoy las 6 entradas tienen `method=0` y el APK mide 195,670,857 bytes)

Escenario 2: **`resources.arsc` sigue guardado sin comprimir** (caso límite)
  Dado el APK generado por `scripts/package_apk.ps1`
  Cuando se lee la entrada `resources.arsc`
  Entonces su `compression_method` es 0
Test: `tests/packaging_tests.rs::test_apk_keeps_arsc_stored` (nuevo)
Estado previo: **pasa** (hoy ya es `method=0`). No es un test rojo: es una guarda de
regresión que protege lo único que este cambio podría romper. Ver "Correcciones previas".

Escenario 3: **`-AllAbis:$false` produce un APK de un solo ABI**
  Dado el texto de `scripts/package_apk.ps1`
  Cuando se inspecciona el bloque que incorpora los ABIs adicionales presentes en `target/`
  Entonces ese bloque está condicionado por `if ($AllAbis)`
Test: `tests/packaging_tests.rs::test_packaging_script_respects_single_abi_flag` (nuevo)
Estado previo: **falla** (hoy no existe esa condición)

Escenario 4: **El juego arranca y se ve igual tras instalar el APK nuevo** (visible)
  Dado el APK recién empaquetado, instalado en el Android TV
  Cuando se abre Blazing Recomp y se carga el juego
  Entonces el juego muestra la pantalla de título y no se cierra
Test: no aplica (revisión humana, requiere dispositivo por `adb`)
Estado previo: no comprobable por el agente (no hay dispositivo conectado)

### 8. Cómo lo comprobaré

- Escenario 1 → comando: `powershell -ExecutionPolicy Bypass -File scripts/package_apk.ps1`
  y después `cargo test --test packaging_tests` → pasa si las 6 entradas `lib/**/*.so`
  tienen `method=8` y el archivo mide ≤ 62,914,560 bytes → no detecta: que Android acepte
  instalar el APK; eso necesita dispositivo.
- Escenario 2 → comando: `cargo test --test packaging_tests` → pasa si `resources.arsc`
  tiene `method=0` → no detecta: que el APK sea instalable en Android 11 o superior, que es
  la razón por la que este criterio existe.
- Escenario 3 → comando: `cargo test --test packaging_tests` (comprobación estática sobre
  el texto del script) → pasa si la condición está presente → no detecta: que la exclusión
  funcione de verdad en ejecución; eso exigiría un empaquetado real con `-AllAbis:$false`
  seguido de `aapt2 dump badging`, que tarda minutos y no forma parte de la suite.
- Escenario 4 ("(visible)") → revisión humana: instalar `target/blazing-recomp-debug.apk`
  en el TV e iniciar el juego → no detecta: rendimiento (el código recompilado no cambia en
  esta propuesta).
- Comprobación general de cierre → `cargo test` (línea base medida hoy: 11/11 en
  `packaging_tests`), `python .agents/skills/diataxis-docs/scripts/validar_docs.py .`,
  `python scripts/extraer_referencias.py --check`,
  `python scripts/diagnostico/auditar_labels.py`.

### 9. Terminado cuando

Se cumplen todos los criterios de aceptación.

### 10. Parar si

2 intentos fallidos seguidos con el mismo criterio. Detenerse, explicar qué se probó y
marcar la propuesta como "discrepada".

### Correcciones previas a presentar

- **Números de línea verificados:** comprobé con `grep -n` cada línea citada de
  `scripts/package_apk.ps1` y de `tests/packaging_tests.rs` después de redactar. Dos
  estaban desviadas (la llamada a `jar` es la 393 y el bucle de ABIs el 371) y quedaron
  corregidas.
- **Tensión de procedimiento detectada y resuelta:** la skill `propuesta-verificable` pide
  escribir los tests antes de editar, pero el pipeline del proyecto (`AGENTS.md`, fase 2)
  prohíbe modificar código antes de la aprobación explícita. Resuelto así: no se escribe
  ningún archivo de test hasta la aprobación, pero el "rojo" de los escenarios 1 y 3 ya
  está demostrado hoy **sin editar nada**, midiendo el APK actual (informe en la sección de
  supuestos). Los tests se crean inmediatamente después de la aprobación y antes de tocar
  el script.
- **Afirmación corregida respecto a mi mensaje anterior:** dije que comprimir no afecta al
  arranque. Es cierto para el ZIP, pero encontré que el repositorio **no define** la opción
  de enlazado de páginas de 16 KB (`max-page-size`) en ningún sitio, y no hay
  `.cargo/config.toml`. Eso es independiente de esta propuesta: la compresión del ZIP no
  cambia la alineación interna del ELF. Queda fuera de alcance y **sin verificar**; no lo
  presento como resuelto.
- **Desviación de la plantilla:** la plantilla recomienda un máximo de 3-4 escenarios y
  aquí hay 4, uno de ellos "(visible)". El cuarto existe porque es lo único que los tests
  no pueden cubrir y la persona debe decidirlo.

---

## Resultado (lo completa el agente después de editar)

### Predicho vs. real

- **Escenario 1 (6 entradas comprimidas + ≤ 60 MiB): coincide.**
  Resultado real: `cargo test --test packaging_tests` → 13 pasaron, 0 fallaron. Comprobación directa
  del APK: 6 entradas `lib/**/*.so`, todas `method=8`, con `comp_size < uncomp_size` (ratios 20.6%
  a 56.1%), y tamaño total de **44,376,905 bytes (42.32 MiB)**, por debajo del presupuesto de
  62,914,560 bytes. En rojo previo falló exactamente por el motivo anunciado (`left: 0, right: 8`
  en `lib/arm64-v8a/libblazing_recomp.so`).
  Método: el anunciado (`scripts/package_apk.ps1` + `cargo test --test packaging_tests`).
- **Escenario 2 (`resources.arsc` sigue sin comprimir): coincide.**
  Real: `method=0`, `comp=40`, `uncomp=40`. Método: el anunciado.
- **Escenario 3 (guarda `-AllAbis`): coincide.**
  Real: el test pasó tras envolver el bucle en `if ($AllAbis)`; en rojo previo falló por el motivo
  anunciado (la primera línea no vacía anterior al `foreach` no era la guarda). Método: el anunciado.
- **Escenario 4 (el juego arranca y se ve igual tras instalar): no pudo comprobarse.**
  Requiere dispositivo por `adb`; no hay ninguno conectado. Pasa a revisión humana.
- **Contraste de magnitud:** la propuesta predijo ~43.9 MB a partir de recomprimir las entradas del
  APK anterior con deflate; el real es 44.38 MB en MB decimales (42.32 MiB), un +1.1% sobre lo
  predicho. La diferencia es esperable porque `jar` no usa exactamente el mismo nivel que la medición previa.

### Archivos modificados

1. `scripts/package_apk.ps1` — retirado `-0` de la llamada a `jar`; `ZIP_STORED` → `ZIP_DEFLATED`
   en el respaldo Python; comentarios y mensajes actualizados; bucle de ABIs adicionales envuelto
   en `if ($AllAbis)` (con reindentación del bloque).
2. `tests/packaging_tests.rs` — constante `APK_SIZE_BUDGET_BYTES`; contrato de
   `test_apk_generation_and_signature` cambiado de STORED + offset 16 KB a DEFLATE + presupuesto;
   nuevos `test_apk_keeps_arsc_stored` y `test_packaging_script_respects_single_abi_flag`.
3. `docs/explicacion/DECISIONES.md` — nuevo ADR 32.
4. `docs/explicacion/arquitectura-port-android.md` — §2.6, paso 4 del pipeline de empaquetado.
5. `docs/propuestas/2026-10-12_compresion-librerias-nativas-apk.md` — esta propuesta (estado y resultado).

¿Coincide con lo anunciado? **Sí**: el paso 6 de la propuesta anunciaba "documentación afectada", y no
se tocó ningún archivo fuera de esta lista.

### Discrepancias

Ninguna en la implementación. La única desviación es de precisión, no de juicio: la estimación de
42.32 MiB frente a los 43.9 MB predichos (+1.1%), atribuible al nivel de compresión efectivo de `jar`.

### Para la próxima vez

Medir el tamaño final comprimiendo con la misma herramienta que ejecutará el empaquetado (`jar`), no
con `zlib` desde Python, para que la predicción de tamaño sea exacta y no aproximada.

### Fuera de alcance (hallazgos que no se arreglaron)

- **Ruta absoluta local versionable en un archivo que no es de esta propuesta:**
  `docs/propuestas/2026-10-12_drain-hotspots-sin-perdida.md` (línea 180) contiene
  `C:/Users/<usuario>/Downloads/...`, lo que viola la regla de portabilidad y privacidad de
  `AGENTS.md` §6 y bloquearía el punto 5 del checklist de pre-commit. El archivo ya existía antes
  de esta tarea y no lo he tocado. | impacto: medio (sube a "Requiere tu decisión")
- El repositorio no fija la alineación de páginas de 16 KB para las bibliotecas nativas
  (`max-page-size`), que Android 15 con páginas de 16 KB puede requerir. | impacto: medio
- El Escenario 1 fija "exactamente 6 entradas `lib/**/*.so`", que es el contrato del APK fat. Un
  empaquetado con `-AllAbis:$false` deja 2 entradas y haría fallar ese test aunque el APK sea
  correcto para su propósito. | impacto: bajo
- El código C11 recompilado pesa ~62 MB por ABI y viaja tres veces: es el 92 % del APK. | impacto: medio
- Las opciones del script de empaquetado (`-AllAbis`, `-SkipBuild`, `-NoStrip`, `-Debug`)
  no están documentadas en `docs/`. | impacto: bajo
- `docs/explicacion/DECISIONES.md` (ADR 15) afirma que el APK pesa 39 MB, cifra que no
  corresponde al estado actual (186.6 MiB). | impacto: bajo

---

## Revisión humana
<!-- El agente SÍ edita esta sección: escribe aquí los escenarios que no se
     pudieron comprobar con código ("(visible)" o "no pudo comprobarse"),
     uno por línea, dejando las marcas ✔ | ✘ | ? sin elegir.
     El agente NO elige marca, NO escribe la nota y NO rellena
     "Detectado solo por el humano": eso es solo de la persona.
     Si no queda nada para revisar: "ninguno: todo se comprobó con tests
     ([cuáles])". -->

- Escenario 4 (el juego arranca y se ve igual tras instalar el APK nuevo): ✔ | ✘ | ?
- Detectado solo por el humano: [lo completa la persona]
