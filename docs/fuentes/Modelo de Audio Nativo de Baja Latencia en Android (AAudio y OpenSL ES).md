# Modelo de Audio Nativo de Baja Latencia en Android (AAudio y OpenSL ES)

## 1. Evolución del Stack de Audio en Android y la Problemática de Latencia

En aplicaciones interactivas en tiempo real (videojuegos, emuladores y sintetizadores), la latencia de ida y vuelta (*round-trip latency*) y la latencia de salida de audio (*output latency*) determinan la respuesta perceptual del usuario. Un retraso excesivo entre la acción del jugador (presionar un botón) y la emisión acústica (el sonido de un ataque en *Fire Emblem*) destruye la inmersión y la precisión temporal.

Históricamente, el sistema operativo Android adoleció de latencias de audio severas (entre 100 ms y 250 ms) debido a las capas de abstracción en Java (`android.media.AudioTrack`), recolección de basura (*Garbage Collection*), conversiones intermedias de formato y mezclas síncronas en el servidor del sistema.

Para resolver estas limitaciones, el subsistema de audio nativo del Android NDK evolucionó a través de tres generaciones de interfaces de bajo nivel:

```
+-------------------------------------------------------------------------------+
|                    EVOLUCIÓN DE LAS APIS DE AUDIO EN ANDROID                  |
|                                                                               |
|  1. Java AudioTrack (Android 1.0+)                                            |
|     [App Java] ---> [JNI] ---> [AudioTrack Native] ---> [AudioFlinger]        |
|     Latencia: 60 - 200 ms. Sujeto a pausas de Garbage Collector.              |
|                                                                               |
|  2. OpenSL ES (Android 2.3+ / API 9+)                                         |
|     [App C/C++] ---> [OpenSL ES Engine] -------------> [AudioFlinger]        |
|     Latencia: 30 - 70 ms. Estándar multiplataforma complejo y obsoleto.       |
|                                                                               |
|  3. AAudio (Android 8.0+ / API 26+) y MMAP HAL                                |
|     [App C/Rust] ---> [AAudio Stream] =================> [ALSA / DSP DMA]     |
|     Latencia: 5 - 15 ms. Acceso directo a memoria compartida sin servidor.   |
|                                                                               |
|  4. Oboe (Google C++ Library)                                                 |
|     Wrapper oficial que autoselecciona AAudio (API 26+) u OpenSL ES (< API 26)|
+-------------------------------------------------------------------------------+
```

---

## 2. Arquitectura Interna del Servidor de Audio: AudioFlinger frente a MMAP Directo

La diferencia fundamental entre el audio convencional de Android y el audio de baja latencia radica en el camino físico que recorren los paquetes de muestras de sonido (*PCM samples*):

### El Camino Tradicional (*Legacy AudioFlinger Path*)
1. La aplicación escribe fragmentos de audio en un búfer mediante llamadas al sistema o colas de eventos.
2. El proceso del servidor de sonido de Android (`audioserver`) aloja el motor de mezcla **AudioFlinger**.
3. Periódicamente, el hilo `MixerThread` de AudioFlinger despierta, toma las muestras de todas las aplicaciones activas, aplica resampleo (*software resampling*) si las frecuencias de entrada no coinciden, convierte formatos numéricos (de punto flotante a enteros de 16 bits), mezcla los flujos en un búfer maestro y lo entrega a la Capa de Abstracción de Hardware (*Audio Hardware Abstraction Layer*, Audio HAL).
4. **Sobrecarga:** Este modelo introduce múltiples cambios de contexto de CPU (*context switches*), transacciones de comunicación inter-proceso (*IPC via Binder*) y colas de almacenamiento intermedias que acumulan entre 30 ms y 80 ms de retraso inevitable.

### El Camino Directo MMAP de Ultra-Baja Latencia (`AAUDIO_SHARING_MODE_EXCLUSIVE`)
Introducido a nivel de arquitectura en Android 8.0 Oreo (API 26) para SoCs con soporte en el kernel y en el HAL (Qualcomm, MediaTek, Samsung Exynos y controladores modernos de audio):
1. **Memoria Compartida por Hardware (*MMAP Memory Mapping*):** La aplicación cliente mapea directamente en su propio espacio de memoria virtual el búfer circular DMA accesible por el circuito integrado de procesamiento de señales de audio (DSP o ALSA driver).
2. **Omisión de AudioFlinger:** La aplicación escribe los datos de audio directamente en la memoria del hardware. El servidor `AudioFlinger` no interviene en la copia ni en la mezcla de los canales durante la reproducción activa.
3. **Modo Exclusivo frente a Modo Compartido:**
   - `AAUDIO_SHARING_MODE_SHARED`: Las aplicaciones comparten un búfer de audio administrado por el sistema. Proporciona latencias moderadas (~25 ms).
   - `AAUDIO_SHARING_MODE_EXCLUSIVE`: La aplicación toma el control monopolístico del punto de acceso hardware de audio. Proporciona la latencia mínima física del dispositivo (inferior a 15 ms, alcanzando entre 3 ms y 8 ms en dispositivos certificados para audio profesional).

---

## 3. Ráfagas Nativas de Hardware (*Burst Sizes*) y Dimensionamiento de Búferes

Para operar sin chasquidos ni pérdidas de muestras acústicas (*buffer underruns* o *glitches*), el dimensionamiento de los búferes debe sincronizarse de manera matemática con el ritmo de transferencia del silicio de audio.

### La Ráfaga Nativa (*Native Frame Burst*)
El chip de audio DSP o el puente ALSA no transfiere las muestras muestra a muestra; en su lugar, consume datos en bloques discretos de tamaño constante denominados **Ráfagas de Cuadro (*Frames Per Burst*)**:
- Dependiendo de la revisión de hardware y del reloj interno del DAC, `framesPerBurst` suele adoptar valores fijos como **96, 128, 192 o 240 muestras**.
- Por ejemplo, a una frecuencia de muestreo canónica de 48,000 Hz, una ráfaga de 192 muestras representa:

$$\text{Duración de la Ráfaga} = \frac{192 \text{ muestras}}{48,000 \text{ muestras/s}} = 0.004 \text{ s} = 4.0 \text{ ms}$$

### Dimensionamiento Óptimo: El Teorema del Doble Búfer
Para que el motor de audio funcione de forma determinista, la capacidad del búfer del flujo (*Buffer Capacity*) debe programarse como un **múltiplo entero estricto de `framesPerBurst`**:

$$\text{Capacidad del Búfer} = k \times \text{framesPerBurst}, \quad \text{donde } k \in \{1, 2, 3\}$$

| Factor $k$ | Modalidad | Latencia Teórica (a 48 kHz, ráfaga de 192) | Comportamiento y Robustez |
| :---: | :--- | :---: | :--- |
| **$k = 1$** | Búfer Simple | $4.0 \text{ ms}$ | **Inestable:** Si el hilo de renderizado o la CPU se retrasa una fracción de microsegundo en entregar la siguiente ráfaga, el DAC se vacía y se produce un chasquido (*underrun* inmediato). |
| **$k = 2$** | Doble Búfer (*Standard*) | $8.0 \text{ ms}$ | **Óptimo para Modo Exclusivo:** El hardware consume el bloque $A$ mientras la aplicación escribe el bloque $B$. Absorbe pequeñas variaciones de planificación de la CPU sin alterar la latencia. |
| **$k = 3$** | Triple Búfer (*Safe Mode*) | $12.0 \text{ ms}$ | **Recomendado para Dispositivos Económicos:** Proporciona un cojín elástico de una ráfaga adicional para absorber fluctuaciones de carga en SoCs de bajo coste (ej. TV Sticks) con procesadores multinúcleo lentos. |

Si una aplicación programa un tamaño arbitrario que no coincide con un múltiplo exacto de la ráfaga (por ejemplo, 500 muestras cuando el hardware opera en pasos de 192), el driver se ve obligado a rellenar fragmentos incompletos o fragmentar las transferencias en dos fases, incrementando drásticamente el consumo de CPU y provocando *underruns* periódicos.

---

## 4. Frecuencias de Muestreo y el Costo del Remuestreo (*Resampling*)

### El Estándar Físico de 48 kHz en Android
Prácticamente la totalidad de los conversores digital-analógico (DAC) integrados en SoCs de Qualcomm, MediaTek, Samsung y Amlogic operan internamente a una frecuencia de muestreo fija de **48,000 Hz (48 kHz)**.

### La Desincronización con el Audio de la Game Boy Advance
El procesador de audio de la Game Boy Advance opera coordinado con la frecuencia del oscilador maestro del sistema ($16.777216 \text{ MHz}$):
- Los canales analógicos clásicos (DMG PSG: canales 1 y 2 de onda cuadrada, canal 3 de onda programable y canal 4 de ruido) sintetizan audio a la frecuencia de salida nominal de la consola, aproximadamente **$32,768 \text{ Hz}$** ($2^{15} \text{ Hz}$).
- Los canales DirectSound A y B (alimentados por DMA a las FIFOs) se programan típicamente mediante los desbordamientos de los temporizadores de GBA (*Timer 0* o *Timer 1*), generando muestras a frecuencias comunes de software como **$16,384 \text{ Hz}$**, **$31,536 \text{ Hz}$** o **$32,768 \text{ Hz}$**.

### Consecuencias Técnicas del Desajuste de Frecuencias
Si una aplicación emite audio a $32,768 \text{ Hz}$ hacia un flujo de Android configurado a $48,000 \text{ Hz}$:
1. **Remuestreo por Software Forzado en el Driver:** Si la aplicación delega la conversión en el sistema operativo, Android invoca algoritmos de resampleo lineal o polinomial en tiempo de ejecución. Esto introduce distorsión armónica por *aliasing* en frecuencias agudas y consume ciclos valiosos en el hilo del sistema.
2. **Remuestreo Nativo de Alta Calidad en Espacio de Usuario:** Para mantener fidelidad acústica sin latencia añadida, el emulador o port nativo debe realizar la interpolación en su propio pipeline en Rust/C antes de entregar las muestras al callback de audio:
   - **Interpolación Lineal:** Rápida pero amortigua las altas frecuencias.
   - **Interpolación Cúbica Hermite o Catmull-Rom:** Excelente compromiso entre coste computacional y fidelidad acústica en CPUs ARM Cortex.
   - **Remuestreadores de Banda Limitada (*Bandlimited Sinc Resampling*):** Calidad audiófila óptima, calculada mediante tablas precalculadas de coeficientes Sinc vectorizadas con NEON.

---

## 5. Modelo de Concurrencia y el Hilo de Audio de Tiempo Real

El subsistema de audio de baja latencia en Android impone un modelo de ejecución basado en **llamadas de retorno asíncronas (*Audio Callbacks*)**, descartando por completo el uso de escrituras bloqueantes (`aaudio_stream_write`).

### Ciclo de Vida del Callback de Audio
1. La aplicación instancia un flujo `AAudioStream` registrando una función de callback:
   ```c
   aaudio_data_callback_result_t AudioDataCallback(
       AAudioStream *stream,
       void *userData,
       void *audioData,
       int32_t numFrames);
   ```
2. El sistema operativo genera un hilo de ejecución exclusivo con prioridad de tiempo real en el planificador del kernel de Linux (`SCHED_FIFO` o prioridad de clase `THREAD_PRIORITY_URGENT_AUDIO`).
3. Cada vez que el controlador DMA del DAC vacía una ráfaga de datos, el kernel despierta de forma inmediata este hilo de audio de alta prioridad, el cual invoca `AudioDataCallback`.

```
                  HILO PRINCIPAL / EMULACIÓN (60 FPS)
                                   |
                Genera muestras GBA (32768 Hz / 60 Hz)
                y realiza remuestreo a 48000 Hz
                                   |
                                   v
             +-------------------------------------------+
             |   Cola Circular Lock-Free SPSC (RingBuf)  |
             +-------------------------------------------+
                                   |
                  HILO DE AUDIO NATIVO (Tiempo Real)
                                   |
                [AudioDataCallback (AAudio / Oboe)]
                                   |
                                   v
              Escribe directamente al búfer DMA de ALSA
```

### Reglas Inviolables de Tiempo Real en el Callback de Audio
Debido a que el callback de audio se ejecuta bajo restricciones de tiempo real estricto (debe completar su ejecución en una ventana de pocos milisegundos para no provocar *underruns* en el DAC), **está estrictamente prohibido realizar las siguientes operaciones dentro del cuerpo del callback**:

1. **Prohibida la Reserva Dinámica de Memoria (`malloc`, `free`, constructores dinámicos en C++ o Rust):**
   Los asignadores de memoria en espacio de usuario emplean bloqueos globales (*mutexes* a nivel de heap) que pueden detener el hilo de audio si otro hilo de la aplicación está asignando memoria en ese instante.
2. **Prohibido el Bloqueo de Mutexes (`pthread_mutex_lock`, `std::sync::Mutex`):**
   Si el hilo de emulación retiene un mutex y es desplanificado por el kernel (*context switch*), el hilo de audio intentará adquirirlo y sufrirá el fenómeno de **Inversión de Prioridad (*Priority Inversion*)**, bloqueándose hasta que el hilo de emulación vuelva a ejecutarse.
3. **Prohibidas las Llamadas de Entrada/Salida (I/O) y Logging:**
   Operaciones como `write()`, llamadas a sockets, lecturas de archivos o escrituras en el log de Android (`__android_log_print` / `logcat`) implican llamadas al sistema del kernel (*syscalls*) que pueden suspender el hilo arbitrariamente.

### La Solución: Búferes Circulares Libres de Bloqueos (*Lock-Free SPSC Ring Buffers*)
La comunicación entre el hilo productor (el emulador de GBA generando tramas de sonido a 60 Hz) y el hilo consumidor (el callback de audio de AAudio drenando datos a ráfagas de 4 ms) debe realizarse obligatoriamente mediante una **Cola Circular de Productor Único y Consumidor Único (*Single-Producer Single-Consumer, SPSC*)**:
- Utiliza índices de lectura y escritura sincronizados mediante variables atómicas con semántica de memoria relajada o de adquisición/liberación (`std::sync::atomic::Ordering::Acquire` / `Release`).
- Cero llamadas al sistema, cero exclusión mutua y cero riesgo de inversión de prioridad.

---

## 6. Selección Práctica entre AAudio, OpenSL ES y Oboe

| Característica | AAudio | OpenSL ES | Oboe |
| :--- | :--- | :--- | :--- |
| **Disponibilidad** | Android 8.0+ (API 26+) | Android 2.3+ (API 9+) | Multiplataforma Android (API 16+) |
| **Soporte MMAP Directo** | Sí (modo exclusivo nativo) | No (pasa siempre por AudioFlinger) | Sí (cuando corre sobre API 26+) |
| **Facilidad de Uso de API** | Código C limpio y plano | Arquitectura verbosa COM-like | C++ idiomático orientado a objetos |
| **Latencia Típica** | $5 - 15 \text{ ms}$ | $30 - 60 \text{ ms}$ | $5 - 15 \text{ ms}$ en API 26+; fallback automático en APIs antiguas |
| **Recomendación para Rust** | Enlaces FFI directos contra `libaaudio.so` | Enlaces a través de `ndk-sys` | Requiere puente C++ o wrapper Rust |

Para *Blazing Recomp*, cuyo target principal son dispositivos modernos con Android 8.0 o superior (arquitecturas ARM64 y TV Sticks actuales con Android 9/10/11), el uso directo de la API nativa de **AAudio en C/Rust** con callback libre de bloqueos proporciona la vía más ligera, robusta y con la menor huella de dependencias externas.
