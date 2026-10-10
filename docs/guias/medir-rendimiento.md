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
1. Una línea de encabezado inicial: `# backend=Vulkan modo=AOT present=Mailbox alpha=Opaque reloj=NNns`
2. Una línea por cada segundo transcurrido con el siguiente formato:
   `t=1s fps=60.0 ejec=2.3/4.1 color=0.8/1.2 dibujo=3.5/6.0 espera=1.2/3.0 subida=0.3 codificacion=1.7 present=1.5 nativos=1520 interprete=0 vacias=12 ventana=1080x2400`

### Significado de los campos
Cada par `A/B` representa el **promedio** y el **peor caso (máximo)** en milisegundos:

| Campo | Unidad | Qué mide |
| :--- | :--- | :--- |
| `t` | segundos | Segundo transcurrido desde el inicio de la sesión (`t=1s`, `t=2s`, ...). |
| `fps` | fotogramas | Cantidad de fotogramas de simulación completados en esa ventana de 1 segundo. |
| `ejec` | ms (prom/máx) | Tiempo de CPU ejecutando la lógica del juego y el hardware emulado. |
| `color` | ms (prom/máx) | Tiempo de CPU convirtiendo píxeles al espacio de color y persistencia LCD (`temporal.compose`). |
| `dibujo` | ms (prom/máx) | Tiempo total de `Compositor::render` (suma de `subida` + `codificacion` + `present`). |
| `espera` | ms (prom/máx) | Tiempo bloqueado esperando un búfer de presentación del swapchain en `get_current_texture`. |
| `subida` | ms (prom) | Fase de subida de la textura de juego y los búferes uniformes a la GPU (`write_texture`/`write_buffer`). |
| `codificacion` | ms (prom) | Fase de adquisición de búfer, grabado de los pases de render y `queue.submit`. |
| `present` | ms (prom) | Fase de `frame.present()`: incluye la espera de VSync **y** la espera de que el compositor libere un búfer del swapchain; suele dominar el coste de `dibujo`. |
| `nativos` | entero | Promedio de bloques C11 nativos recompilados ejecutados por fotograma. |
| `interprete` | entero | Promedio de pasos de instrucción en el intérprete de respaldo por fotograma. |
| `vacias` | entero | Vueltas del bucle de eventos sin avance de fotograma ni redibujado durante ese segundo. |
| `ventana` | píxeles | Resolución física de la ventana (`ancho x alto`), distinguiendo portrait de landscape. |

### Líneas de calibración de presentación

La calibración de presentación está **desactivada por defecto**: la medición en tablet mostró que solo la
reducción de resolución baja el coste de `present` (51,4 ms → 2,9 ms; 18 → 53-60 FPS) y se decidió
preservar la nitidez nativa, así que el barrido no tenía nada que adoptar y se retiró del arranque
(constante `CALIBRAR_PRESENTACION` en `src/platform.rs`). Con la calibración **activada**, los primeros
~12-15 s de sesión recorren configuraciones del *swapchain* (latencia de fotogramas, modo de presentación
y una sonda de resolución reducida). Esas líneas empiezan por `#` y acompañan a las ventanas `t=` que
describen: las líneas `t=` que siguen a un `# tramo` pertenecen a esa configuración.

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
