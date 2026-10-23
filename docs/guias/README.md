---
tipo: guia-practica
verificado: si
---

# Guías Prácticas de Desarrollo y Operación

Este directorio agrupa las guías prácticas paso a paso para tareas concretas de compilación, diagnóstico, empaquetado y medición en **`blazing-recomp`**, siguiendo el cuadrante de **Guías Prácticas** (*How-to Guides*) del estándar **Diátaxis**.

---

## Índice de Guías Disponibles

| Guía Práctica | Objetivo |
| :--- | :--- |
| **[`compilar-y-empaquetar-apk.md`](compilar-y-empaquetar-apk.md)** | Pasos para compilar las bibliotecas nativas C11/Rust con NDK, procesar recursos con AAPT2, empaquetar, alinear e instalar el Fat APK en Android. |
| **[`mapear-labels-aot.md`](mapear-labels-aot.md)** | Pasos para desensamblar código Thumb de la ROM, descubrir puntos de entrada, registrar nuevas funciones en `blazing.labels.toml` y regenerar código C11. |
| **[`medir-rendimiento.md`](medir-rendimiento.md)** | Pasos para registrar, extraer y analizar la telemetría por fases (`ejec`, `ppu`, `apu`, `dibujo`, `present`) generada por el runtime en Android. |
