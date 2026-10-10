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
1. Una línea de encabezado inicial: `# backend=Vulkan modo=AOT reloj=NNns`
2. Una línea por cada segundo transcurrido con el siguiente formato:
   `t=1s fps=60.0 ejec=2.3/4.1 color=0.8/1.2 dibujo=3.5/6.0 espera=1.2/3.0 nativos=1520 interprete=0 vacias=12 ventana=1080x2400`

### Significado de los campos
Cada par `A/B` representa el **promedio** y el **peor caso (máximo)** en milisegundos:

| Campo | Unidad | Qué mide |
| :--- | :--- | :--- |
| `t` | segundos | Segundo transcurrido desde el inicio de la sesión (`t=1s`, `t=2s`, ...). |
| `fps` | fotogramas | Cantidad de fotogramas de simulación completados en esa ventana de 1 segundo. |
| `ejec` | ms (prom/máx) | Tiempo de CPU ejecutando la lógica del juego y el hardware emulado. |
| `color` | ms (prom/máx) | Tiempo de CPU convirtiendo píxeles al espacio de color y persistencia LCD (`temporal.compose`). |
| `dibujo` | ms (prom/máx) | Tiempo de GPU en `Compositor::render` (subida de texturas, shaders y presentación). |
| `espera` | ms (prom/máx) | Tiempo bloqueado esperando un búfer de presentación del swapchain en `get_current_texture`. |
| `nativos` | entero | Promedio de bloques C11 nativos recompilados ejecutados por fotograma. |
| `interprete` | entero | Promedio de pasos de instrucción en el intérprete de respaldo por fotograma. |
| `vacias` | entero | Vueltas del bucle de eventos sin avance de fotograma ni redibujado durante ese segundo. |
| `ventana` | píxeles | Resolución física de la ventana (`ancho x alto`), distinguiendo portrait de landscape. |
