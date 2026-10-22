---
tipo: guia-practica
verificado: si
---

# Cómo medir el rendimiento por fase del fotograma

Esta guía práctica explica cómo recopilar y leer el archivo de telemetría de rendimiento `blazing-perf.txt` generado por la aplicación en dispositivos Android.

---

## Requisitos

- Tener instalada la aplicación Blazing Recomp en el dispositivo (móvil, tablet, Android TV o emulador).
- Haber concedido permisos de almacenamiento a la aplicación (para acceso a la carpeta `Download/`).
- Contar con un explorador de archivos en el dispositivo o conexión `adb`.

---

## Pasos

### 1. Iniciar la medición durante el juego
1. Inicia **Blazing Recomp** con la copia verificada de la ROM oficial y el BIOS.
2. Juega durante al menos 30 segundos en la escena que desees perfilar (mapa táctico, combate o animación).
3. Cada segundo de juego activo, el motor acumula los tiempos de cada fase y escribe una línea en el archivo de texto.

### 2. Extraer el archivo `blazing-perf.txt`
El archivo se guarda automáticamente en una de las siguientes ubicaciones:
- **Ruta principal:** `Download/blazing-perf.txt` (o `/sdcard/Download/blazing-perf.txt`).
- **Ruta interna de la aplicación:** `Android/data/com.blazingrecomp.game/files/blazing-perf.txt`.

Para extraer el archivo mediante ADB:
```bash
adb pull /sdcard/Download/blazing-perf.txt .
```

O ábrelo directamente en el dispositivo con Files de Google u otro explorador de archivos.

---

## Comprobar el resultado

Para validar que la medición se registró correctamente, abre `blazing-perf.txt` y comprueba que contenga:
1. Una línea de encabezado inicial: `# backend=Vulkan modo=AOT present=Mailbox alpha=Opaque reloj=NNns ppu=recto-3`
2. Una línea por cada segundo transcurrido con el siguiente formato:
   `t=45s fps=60.0 ejec=13.6/21.9 ppu=7.7 ppubg=3.3 ppuspr=1.1 ppucomp=3.3 apu=1.5 lecturas=6581 escrituras=4206 color=1.2/1.3 dibujo=7.0/15.5 espera=0.0/0.0 subida=0.5 codificacion=1.2 present=5.4 presentmax=14.0 nativos=4945 interprete=2 vacias=1 ventana=1280x720 pump=2 pumpm=480 formato=2`

### Significado de los campos
Cada par `A/B` representa el **promedio** y el **peor caso (máximo)** en milisegundos:

| Campo | Unidad | Qué mide |
| :--- | :--- | :--- |
| `t` | segundos | Segundo transcurrido desde el inicio de la sesión (`t=1s`, `t=2s`, ...). |
| `fps` | fotogramas | Cantidad de fotogramas de simulación completados en esa ventana de 1 segundo. |
| `ejec` | ms (prom/máx) | Tiempo de CPU ejecutando la lógica del juego y el hardware emulado (incluye la PPU y el APU). |
| `ppu` | ms (prom) | Tiempo de CPU de la PPU en software. Desde el 2026-10-12 es **exactamente la suma de sus tres partes**: `ppu = ppubg + ppuspr + ppucomp`, sin contador propio. Si la suma se desvía de `ppu` más de 0,1 ms (la resolución de impresión), hay un contador sin declarar. |
| `ppubg` | ms (prom) | Renderizado de fondos (modos de texto y bitmap), incluido el desplazamiento de las líneas. |
| `ppuspr` | ms (prom) | Extracción y dibujado de sprites, incluidas las transformaciones afines de la OAM. |
| `ppucomp` | ms (prom) | Composición de la línea: prioridad de capas, ventana, mosaico y mezclas alfa y semitransparentes. |
| `apu` | ms (prom) | Muestreo de audio de los canales que avanzan en esa ventana. |
| `lecturas` / `escrituras` | enteros | Lecturas y escrituras del bus de memoria emulado por fotograma (promedio de la ventana). |
| `color` | ms (prom/máx) | Tiempo de CPU convirtiendo píxeles al espacio de color y persistencia LCD (`temporal.compose`). |
| `dibujo` | ms (prom/máx) | Tiempo total de `Compositor::render` (suma de `subida` + `codificacion` + `present`). |
| `espera` | ms (prom/máx) | Tiempo bloqueado esperando un búfer de presentación del swapchain en `get_current_texture`. |
| `subida` | ms (prom) | Fase de subida de la textura de juego y los búferes uniformes a la GPU (`write_texture`/`write_buffer`). |
| `codificacion` | ms (prom) | Fase de adquisición de búfer, grabado de los pases de render y `queue.submit`. |
| `present` | ms (prom) | Fase de `frame.present()`: incluye la espera de VSync **y** la espera de que el compositor libere un búfer del swapchain; suele dominar el coste de `dibujo`. |
| `presentmax` | ms (máx) | Peor `present` de esa ventana. Un valor muy por encima del resto de la sesión señala un tirón del compositor de Android (u otra pausa del sistema), no trabajo del motor. |
| `nativos` | entero | Promedio de bloques C11 nativos recompilados ejecutados por fotograma. |
| `interprete` | entero | Promedio de pasos de instrucción en el intérprete de respaldo por fotograma. |
| `vacias` | entero | Vueltas del bucle de eventos sin avance de fotograma ni redibujado durante ese segundo. |
| `ventana` | píxeles | Resolución física de la ventana (`ancho x alto`), distinguiendo portrait de landscape. |
| `pump` | llamadas | Promedio de llamadas a `pump_audio` por fotograma (bombeo de audio por evento, desde el 2026-10-14). |
| `pumpm` | muestras | Promedio de muestras de audio generadas por fotograma. |
| `formato` | versión | Versión del contrato con la que se escribió la línea. Hoy `2`. Es el **sello**, va al final y permite distinguir una línea nueva de una vieja sin adivinar por los campos que trae. |

La lista de campos, sus tipos y su orden **no se mantienen a mano**: los declara
[`telemetria/esquema.txt`](../../telemetria/esquema.txt), del que beben el motor
([`src/telemetria.rs`](../../src/telemetria.rs)) y el analizador de Python. Si esta tabla y el
esquema se desfasan, gana el esquema. El analizador busca cada campo **por clave**, así que el
orden de la línea deja de importar; y una línea sin sello (las anteriores al 2026-10-14) se lee
en modo tolerante: usa los campos que encuentra y no echa de menos los que aún no existían.

Sobre `reloj=` del encabezado: es el **coste medido de una lectura de `Instant::now()` del port**
(promedio de 10.000 lecturas al arrancar, [`src/perf.rs`](../../src/perf.rs)). En estos dispositivos
ronda 1,4-2,2 µs, tres órdenes de magnitud por encima de lo habitual, y sirve para saber cuánto cuesta
cronometrar la propia telemetría. **No** mide el contador del sistema con el que el motor cronometra
las fases de la PPU, así que no sirve para comparar el coste de ese reloj entre versiones; medirlo en el dispositivo es el
objetivo de [`2026-10-13_el-reloj-de-la-ppu-en-el-dispositivo.md`](../propuestas/2026-10-13_el-reloj-de-la-ppu-en-el-dispositivo.md).

### Leer el informe del analizador

`python scripts/diagnostico/analizar_telemetria.py --archivo blazing-perf.txt` resume la sesión.
Con el contrato versión 2, el informe separa dos cosas que antes se confundían en un solo número:

- **peor media de segundo**: el mayor promedio de `ejec` (o de `dibujo`) entre las ventanas de 1 s.
  Es comparable con la media de la sesión y sirve para ver un segundo entero que fue mal.
- **peor fotograma**: el mayor de los segundos componentes del par `A/B`. Un pico aislado de un
  solo fotograma (un tirón del compositor) se ve aquí y no en la media del segundo.

Si el analizador encuentra algo que no encaja con `telemetria/esquema.txt` —un campo que no
conoce, un valor que no encaja con su tipo o un campo que falta en una línea sellada—, no lo
descarta: añade una sección `ANOMALÍAS DE FORMATO EN LA LÍNEA DE SEGUNDO` con el motivo y el
recuento, y la línea sigue contando para las medias. Un campo desconocido suele significar que el
APK es más nuevo que el script; un campo que falta, que el script es más nuevo que el APK.

Para ver exactamente lo que el lector entendió, `--volcar` emite un TSV con una fila por campo:

```bash
python scripts/diagnostico/analizar_telemetria.py --archivo blazing-perf.txt --volcar
```

Es el mismo volcado que usa el test de oro (`tests/telemetria_contrato_tests.rs`) para comprobar,
campo a campo, que lo que el motor escribió es lo que el analizador leyó.

### Líneas de perfilado del intérprete

Además de las líneas `t=`, cada 2 segundos (cuando el segundo medido es par) el motor escribe tres
líneas que empiezan por `#`: dos describen qué código está ejecutando el intérprete de respaldo y una
reparte las líneas de la PPU entre los caminos de composición.

| Línea | Significado |
| :--- | :--- |
| `# hotspots top=ADDR (N) ...` | Las 6 direcciones de código más ejecutadas en el intérprete desde el volcado anterior, con su número de ejecuciones. |
| `# regiones rom=I/N iwram=I/N ewram=I/N bios=I/N` | Ejecución **acumulada desde el arranque de la sesión**, por región de memoria. Cada par es `I` = pasos de intérprete y `N` = bloques nativos AOT. |
| `# ppucamino rectas=N mezcla=N ventana=N semi=N inertes=N efecto=N efalpha=N efbrillo=N efoscuro=N (rectas=X%)` | Líneas de la PPU **acumuladas desde el arranque de la sesión**, repartidas por camino de composición: `rectas` = vía rápida (tramos de control constante), y los descartes por efectos de color que necesitan las dos capas (`mezcla`), ventana de sprites (`ventana`) o sprite translúcido (`semi`). `inertes` y `efecto` desglosan las `rectas`: las que lo son porque el efecto es la identidad, y las que aplicaron un aclarado u oscurecido real. `efalpha`, `efbrillo` y `efoscuro` reparten las líneas con efecto real por modo. |
| `# ppu_hw gpu=N sw=N modo0=N win=N blend=N spr_total=N spr_8bpp=N spr_afin=N nativa=N` | Censo de **la ventana de 1 s** a la que acompaña: `gpu` = fotogramas dibujados en la GPU, `sw` = fotogramas que ebieron por la subida del framebuffer por software, `modo0`/`win`/`blend` = fotogramas en Modo 0, con ventanas de hardware activas y con efectos de mezcla, `spr_total`/`spr_8bpp`/`spr_afin` = promedios de sprites activos, de 8bpp y afines, y `nativa` = cuántos de esos fotogramas los dibujó la **PPU nativa** (los que fueron por GPU con el shim no cuentan). |

Sobre `# ppucamino`: las **cuatro primeras** cuentas (`rectas`, `mezcla`, `ventana`, `semi`) suman las
líneas dibujadas y cada línea cae en una sola razón —la primera que impide usar la vía rápida—, así
que el reparto no deja líneas sin explicar y siempre cierra en múltiplos de 160 (las líneas de un
fotograma). Las demás cuentas son informativas y se solapan con esas cuatro: `inertes` y `efecto`
son subconjuntos de `rectas`, y `efalpha + efbrillo + efoscuro = efecto + mezcla` (todas las líneas
con efecto real, apliquen el efecto por la vía rápida o se descarten). El porcentaje `rectas=X%` se
calcula sobre el total acumulado, no sobre la ventana: para ver la escena actual, se restan dos
líneas consecutivas. Un `rectas=0%` con `mezcla` creciendo significa que la escena necesita el alpha
real (modo 1) y que la vía rápida no se está ejecutando; si en cambio crece `efoscuro` o `efbrillo`
mientras `rectas` sube, el efecto es un oscurecido o aclarado que la vía rápida sí aplica. Un
`inertes` creciente con `rectas` subiendo son desvanecidos a nivel cero: efecto configurado que no
cambia ningún píxel.

La **ventana de sprites** es la primera razón que se mira, antes que el efecto de color: una línea
con esa ventana activa se dibuja píxel a píxel, porque el atajo solo corta la línea por los bordes de
WIN0 y WIN1 y no puede aplicar la máscara de la ventana de sprites, que depende de dónde estén los
sprites ([`2026-10-21_ventana-obj-camino-rapido.md`](../propuestas/2026-10-21_ventana-obj-camino-rapido.md)).
Por eso, en una escena con la ventana activa `ventana` crece y `mezcla` no: esas líneas se anotan por
la ventana aunque también tengan un alpha real, y `efalpha` solo cuenta las líneas con alpha real a
las que la ventana no había descartado antes.

El campo `ppu=` del encabezado es la marca de compilación de ese camino
(`PPU_CAMINO_VERSION` del motor) y sirve para saber qué renderizador produjo el registro
(`ppu=recto-3` es la versión actual, con la ventana de sprites ya aplicada; `recto-2` trae el mismo
reparto pero con el atajo sin la ventana; `recto-1` no tiene los campos nuevos).

Sobre `# hotspots top=`: solo se consumen las direcciones que aparecen en la línea; las que no entran
en el `top` conservan su contador para el volcado siguiente, de forma que la suma de todas las líneas
del archivo equivale al total contado. Las direcciones pertenecen a ROM (`0x08......`), IWRAM
(`0x03......`), EWRAM (`0x02......`) o BIOS (`0x00......`).

Sobre `# regiones`: es un acumulado de la sesión, no una ventana, así que en un archivo con varias
sesiones solo vale la última línea. `interprete` cuenta pasos de instrucción y `nativos` cuenta
bloques, de modo que dividir ambos da una fracción de *eventos de despacho*, no de instrucciones
ejecutadas. Una región con muchos pasos de intérprete y cero bloques nativos (por ejemplo
`iwram=122117607/0`) señala código de esa región que la tabla AOT no cubre.

Para resumir ambas líneas sin hacerlo a mano:

```bash
python scripts/diagnostico/analizar_telemetria.py --archivo blazing-perf.txt
```

El informe imprime el desglose regional acumulado y el ranking de hotspots. Con un registro anterior
al 2026-10-12 (sin líneas `# regiones`) el analizador lo avisa y omite esa sección.

### Líneas de calibración de presentación

La calibración de presentación está **activa** mientras dure el experimento
[`2026-10-14_reactivar-la-calibracion-de-presentacion.md`](../propuestas/2026-10-14_reactivar-la-calibracion-de-presentacion.md)
(constante `CALIBRAR_PRESENTACION` en `src/platform.rs`); fuera de él queda **desactivada por defecto**,
porque la medición en tablet mostró que solo la reducción de resolución baja el coste de `present`
(51,4 ms → 2,9 ms; 18 → 53-60 FPS) y se decidió preservar la nitidez nativa. Con la calibración activa,
los primeros ~12-15 s de sesión recorren configuraciones del *swapchain* (latencia de fotogramas, modo de
presentación y una sonda de resolución reducida). Esas líneas empiezan por `#` y acompañan a las ventanas
`t=` que describen: las líneas `t=` que siguen a un `# tramo` pertenecen a esa configuración.

| Línea | Significado |
| :--- | :--- |
| `# tramo idx=N lat=… modo=… escala=…` | Empieza la medición de una configuración (`lat` = `desired_maximum_frame_latency`, `modo` = `present_mode`, `escala` = resolución de render relativa). |
| `# resultado idx=N present=…ms fps=…` | Promedio de esa configuración al cerrarse (3 ventanas de 1 s). |
| `# calibracion reiniciada (cambio de orientacion)` | La ventana cambió de tamaño: el recorrido se reinicia, porque el coste de composición depende de la rotación. |
| `# elegido lat=… modo=… escala=… (motivo)` | Configuración adoptada y el motivo. Las etiquetas `referencia` y `sonda` marcan los tramos que **no** se adoptan. |

Dos de los tramos son **sondas de diagnóstico**: se miden, pero nunca se adoptan.

- `modo=Immediate` (presentar sin VSync): sirve para saber si el bloqueo de `present` es de
  sincronización o trabajo real del compositor. Adoptarlo produciría *tearing*.
- `escala=0.50` (renderizar a media resolución): sirve para saber si el coste escala con los píxeles.
  Bajar la resolución de forma permanente es una decisión de nitidez que hay que tomar aparte, así que
  la calibración solo aporta el dato.

La calibración no altera el resto del formato: el encabezado y las líneas `t=` siguen igual.

El encabezado informa además del backend realmente activo (`backend=Vulkan` o `backend=OpenGL`). Durante el
experimento de presentación la aplicación prefiere OpenGL ES, así que **solo deben compararse registros
del mismo backend**: un log con `backend=OpenGL` mide el camino de OpenGL ES, no el de Vulkan.

---

## Probar la PPU nativa en el dispositivo

La **PPU nativa** compone el cuadro píxel a píxel en la GPU, con la aritmética de la PPU
original de la GBA (mezcla alfa, aclarar, oscurecer y ventanas). Verifica contra la PPU de
referencia con **0 diferencias de 38.400 píxeles** en los dos cuadros de aceptación
([`2026-10-19_ppu-nativa-fase-1.md`](../propuestas/2026-10-19_ppu-nativa-fase-1.md)).

Es la ruta de dibujo **por defecto**: un APK recién instalado ya compone con ella, sin tocar
nada ([`2026-10-21_ppu-nativa-activa-por-defecto.md`](../propuestas/2026-10-21_ppu-nativa-activa-por-defecto.md)).
Para volver al shim —y comparar A/B en el propio dispositivo— se crea un archivo de marca,
sin recompilar:

1. Crea un archivo vacío llamado `ppu-nativa-off.flag` en el almacenamiento del dispositivo.
2. Deja la app en marcha: la marca se relee **una vez por segundo**, así que el cambio se ve
   sin reiniciar.
3. Para volver a la PPU nativa, borra el archivo: el control vuelve a ella en el siguiente tick.

> El archivo `ppu-nativa.flag` de las pruebas anteriores **ya no se lee**: encendía la PPU
> nativa cuando la ruta por defecto era el shim. Si sigue en el almacenamiento, no hace nada.

Rutas candidatas, por orden de búsqueda. La primera es la única que no depende de permisos:

| Ruta | Notas |
| :--- | :--- |
| `/sdcard/Android/data/com.blazingrecomp.game/files/ppu-nativa-off.flag` | Directorio externo propio de la app: **siempre** accesible, sin permisos. |
| `/sdcard/Download/ppu-nativa-off.flag` | Requiere que la app tenga acceso al almacenamiento compartido. |
| `/storage/emulated/0/Download/ppu-nativa-off.flag` | Idem, ruta emulada. |

La lista completa la fija `resolve_search_directories` en
[`src/storage.rs`](../../src/storage.rs): a las tres anteriores les siguen la subcarpeta
`/sdcard/Download/GBA`, las rutas relativas `Download` y `roms` (desarrollo en host), las
alternativas de emulador (`/mnt/sdcard/Download`, `/storage/self/primary/Download`) y, la última,
el directorio interno privado de la app.

Con `adb`:

```bash
# Volver al shim
adb shell touch /sdcard/Android/data/com.blazingrecomp.game/files/ppu-nativa-off.flag
# Volver a la PPU nativa
adb shell rm /sdcard/Android/data/com.blazingrecomp.game/files/ppu-nativa-off.flag
```

**Cómo se sabe que está activa.** En el Logcat aparece `PPU nativa: ACTIVADA por defecto al
arrancar` (o `PPU nativa: ACTIVADA (marca de apagado retirada)` si se acaba de borrar el
archivo), y en `blazing-perf.txt` la línea `# ppu_hw` trae el campo `nativa=`, con los
fotogramas que dibujó la PPU nativa en esa ventana:

```text
# ppu_hw gpu=60 sw=0 modo0=0 win=0 blend=1 spr_total=12 spr_8bpp=0 spr_afin=0 nativa=60
```

`nativa=60` (o cercano a 60) sobre `gpu=60` es la prueba de que la PPU nativa es quien está
dibujando; con el shim, `nativa` valdría 0. La marca también queda registrada en el archivo
como `# ppu_nativa=ON` y `# ppu_nativa=OFF` en el momento del cambio.

**Lo que esta prueba NO mide.** La PPU nativa no sustituye al rasterizado por software: el
motor sigue rasterizando en CPU, así que el **FPS no mejora** por encenderla. Lo que se
comprueba es la **imagen**: escenas con mezcla alfa, ventanas de hardware y sprites
translúcidos, que es justo lo que el shim no implementaba.

### Si el dispositivo no puede con la PPU nativa

La construcción del pipeline de la PPU nativa va dentro de un **guardián de errores** de
`wgpu`: si el dispositivo rechaza algún recurso, la app **no se cae**, lo anota en el Logcat
y sigue con el shim durante el resto de la sesión:

```text
PPU nativa: no se pudo construir el pipeline (PPU nativa: Validation Error …); se sigue con el shim
```

En ese caso la telemetría mostrará `nativa=0` aunque la PPU nativa esté activa por defecto: la
app vuelve a leer la marca cada segundo, pero no reintenta la construcción (para no pagar dos
veces el coste por fotograma).

Si en cambio la app muestra «CRASH CRÍTICO» con el mensaje `wgpu error: Validation Error`, el
error no vino de la construcción de la PPU nativa sino de otro recurso de GPU. La causa más
frecuente fue la PPU nativa pidiendo un *storage
buffer* visible en la etapa de vértices, algo que el estándar de OpenGL ES permite no tener
(`DownlevelFlags(VERTEX_STORAGE)`): el registro del crash lo dice con «Binding N entry is invalid».
Hoy **ningún** recurso de la PPU nativa exige esa capacidad —la geometría de los sprites viaja
como atributos de instancia—, así que un dispositivo sin Vulkan (que acaba en OpenGL ES) también
puede usarla. El arreglo está en
[`2026-10-21_ppu-nativa-sin-storage-en-vertices.md`](../propuestas/2026-10-21_ppu-nativa-sin-storage-en-vertices.md).

Si el crash persiste, mira primero el backend que está usando el dispositivo: el encabezado de
`blazing-perf.txt` informa `backend=Vulkan` o `backend=OpenGL`, y el Logcat (`adb logcat -s
BlazingRecomp`) dice qué backend se inicializó. La ruta de respaldo de Vulkan a OpenGL ES es
automática (`src/platform.rs`), así que un dispositivo sin Vulkan no debería quedarse sin GPU.

---

## Comparar dos versiones sin engañarse

El arnés del motor (`recomp runc`) ejecuta el juego sin pantalla y en segundos, así que sirve para
descartar una idea antes de empaquetar el APK. La comparación solo vale si es **pareada**:

1. **Dos binarios, una sola imagen.** Compilar cada variante
   (`cargo build --release -p recomp` y `cargo build --release -p recomp --features gba-core/perf-counters`)
   y copiar cada binario aparte. Las dos variantes deben correr **el mismo `.dll`**; si se regenera la
   imagen entre una y otra, lo que se mide incluye la imagen.
2. **Correr desde la raíz del port**, porque la imagen se busca como `out/<rom>-bios.dll` relativa al
   directorio actual:
   ```bash
   ../gba-recomp/target/release/recomp.exe runc "roms/1575 - Fire Emblem (E)(Independent).gba" \
     --bios roms/gba_bios.bin --frames 7200
   ```
3. **Intercalar las variantes** (A, B, A, B…) y no correr cada una en bloque: entre corridas idénticas de
   7200 fotogramas en la misma máquina la dispersión medida fue de 7589 a 10468 ms (**±13 %**), y la
   deriva de la máquina se cancela con pares intercalados.
4. **Cinco pares o más**, y comparar **la mediana de las diferencias pareadas**, no la mediana de cada
   variante por separado: con una dispersión mayor que el efecto, las medianas sueltas pueden cambiar de
   signo (medido: +0,4 % contra una mediana pareada de −3,6 % para el mismo par de binarios).
5. **Comprobar el hash antes del tiempo**: las dos variantes tienen que imprimir el mismo `hash:` y el
   mismo `native_blocks:` en cada longitud de corrida. Si el hash cambia, la comparación no mide
   rendimiento: mide otro juego.
6. **Medir la feature que se publica.** El APK activa `perf-counters`
   ([`Cargo.toml`](../../Cargo.toml)), así que un binario compilado sin esa feature no representa el coste
   del APK: es la variante de control que dice cuánto cuesta la instrumentación.
7. **Mirar qué camino de plataforma toma el código que se mide.** El cronómetro de la PPU solo es
   `CNTVCT_EL0` bajo `aarch64`: en el arnés de host, en `armeabi-v7a` y en `x86_64`, `perf_clock` cae en
   `Instant::now()`, que aquí cuesta ~140 ns por lectura —640 por fotograma, unos 0,09 ms, casi un 10 % del
   fotograma de este arnés—. Una diferencia medida en host con la feature encendida **incluye ese reloj**, y
   no es lo que paga el APK de `arm64-v8a`.

Por debajo de un 5 % de diferencia este arnés no concluye: para eso está la telemetría del dispositivo.Y si lo que se quiere medir es el coste del cronómetro del motor en el objetivo real, este arnés no sirve: para eso está la sonda de [`2026-10-13_el-reloj-de-la-ppu-en-el-dispositivo.md`](../propuestas/2026-10-13_el-reloj-de-la-ppu-en-el-dispositivo.md).

---

## Fotogramas dorados: regresiones de píxel de la PPU por escena

El arnés de fotogramas dorados (`scripts/diagnostico/fotogramas_dorados.py`) detecta **qué píxel cambió**, **en qué escena y fotograma**, y **qué región de la pantalla** ocupó, en lugar de solo indicar que un hash global difiere. Es la red de fidelidad que el arnés de rendimiento no tiene.

### Uso en tres pasos

```bash
# 1. Capturar los dorados de las escenas del manifiesto (se ejecuta una vez por binario de confianza)
python scripts/diagnostico/fotogramas_dorados.py capturar \
    --bin ../gba-recomp/target/release/recomp.exe \
    --rom "roms/1575 - Fire Emblem (E)(Independent).gba" \
    --bios roms/gba_bios.bin

# 2. Hacer el cambio en la PPU, reconstruir, y comparar contra los doradosnpython scripts/diagnostico/fotogramas_dorados.py comparar \
    --bin ../gba-recomp/target/release/recomp.exe \
    --rom "roms/1575 - Fire Emblem (E)(Independent).gba" \
    --bios roms/gba_bios.bin

# 3. Si falla, el informe dice cuántos píxeles, en qué columna/fila, qué caja ocupan,
#    y escribe un PNG `entrada-avance_<k>.diff.png` con las diferencias en rojo.
```
n### Contrato de códigos de salida (los tres subcomandos)

| Código | Significado |
| :--- | :--- |
| 0 | Sin diferencias (o captura correcta) |
| 1 | **Diferencias de píxel: es una regresión** |
| 2 | Entrada inválida o dorados desactualizados (ROM/BIOS/guion cambiaron); **no hay veredicto** |
n### Qué informa `comparar` cuando falla

```text
escena entrada-avance
  ancla 600: OK (ec8c6a01f9add6ff)
  ancla 1800: DIFIERE 93 px | primero columna=119 fila=12 | caja x0-239 y12-23 | filas 12-23 | tiles 8x8 afectados 28/600 | mapa: out/doradas/entrada-avance_1800.diff.png
  ...

4 ancla(s) difieren, 311 píxeles en total: entrada-avance@1800 (93 px), ...
```
n### Manifiesto de escenas (`scripts/diagnostico/escenas_doradas.toml`)
nVersionado y editable sin propuesta; solo lista `nombre`, `script` (relativo a la carpeta) y `anclas`:
```toml
version = 1

[[escena]]
nombre = "entrada-avance"
script = "entrada_avance.input"
anclas = [600, 1800, 3600, 5400, 7200]
```
n### Blindaje por mutación (se ejecuta antes de cerrar cualquier propuesta que toque la PPU)
nM1 — `alpha_blend` (`+8` → `+7`): **detecta 311 píxeles en 4 anclas** (600 OK; 1800/3600/5400/7200 difieren).
M2 — `in_window_range` (rango semiabierto → cerrado): **0 diferencias** (los anclas actuales no ejercitan ventanas).
nLos archivos del motor se restauran byte a byte en un bloque `finally`, sin comandos destructivos de Git.
n### Dónde viven los dorados (no versionados)
n`out/doradas/` (cubierto por `/out/` en `.gitignore`):
- `entrada-avance_<k>.ppm` — fotograma ancla (PPM P6 240×160)
- `manifesto.json` — hashes de ROM, BIOS, guion, biblioteca `.dll` y anclas
- `entrada-avance_<k>.diff.png` — mapa de diferencias si `comparar` falla
nSolo se versionan la herramienta, `escenas_doradas.toml`, `entrada_avance.input` y los tests.
