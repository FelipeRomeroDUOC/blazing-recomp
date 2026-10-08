# AGENTS.md — Blazing Recomp

Directrices, arquitectura y reglas operativas para agentes de IA que trabajen en el proyecto **Blazing Recomp**.

---

## 1. Visión y Propósito del Proyecto

- **Nombre:** Blazing Recomp (`blazing-recomp`)
- **Objetivo:** Port nativo del juego **Fire Emblem: The Blazing Blade (GBA)** con destino de ejecución nativa en **Android**.
- **Fundamento tecnológico:** Basado en el pipeline de recompilación estática AOT (*Ahead-Of-Time*) de `gba-recomp`:
  - Lógica del juego traducida a C11 y compilada nativamente para la arquitectura del dispositivo objetivo (**ARM64-v8a** en Android).
  - Runtime y modelo de hardware (PPU, APU, bus de memoria, DMA, timers) provistos por `gba-core` en Rust.
  - Cero emulación tradicional de la CPU en tiempo de ejecución.

---

## 2. Plataforma Objetivo: Android Nativo

1. **Arquitectura:**
   - Target principal: `aarch64-linux-android` (ARM64).
   - Secundario / Emuladores: `x86_64-linux-android`.
2. **Capa Nativa (NDK & Rust):**
   - El código Rust y C recompilado se empaquetará como biblioteca compartida (`.so`) enlazada vía JNI o `android-activity` / `winit`.
3. **Renderizado & Pantalla:**
   - Backend gráfico compatible con Android: **Vulkan** o **OpenGL ES 3.0** a través de `wgpu`.
   - Soporte para diferentes relaciones de aspecto y resoluciones de smartphones modernos (incluyendo escalado de píxeles nítido y posibles mejoras panorámicas).
4. **Controles & Entrada:**
   - Soporte para mandos físicos (Bluetooth y USB Type-C vía Gamepad API de Android).
   - Capa de controles táctiles virtuales superpuestos en pantalla (*touch overlay* personalizable).
   - Opcional: Integración de interacción táctil directa en los menús de batalla del juego si es viable.
5. **Rendimiento & Batería:**
   - La ejecución nativa debe optimizar el consumo de batería; sin bucles de sincronización activa (*busy-waiting*) innecesarios.
   - Pacing de fotogramas estricto a 60 FPS coordinado con el `Choreographer` de Android o VSync de la pantalla.

---

## 3. Datos del Juego y Seguridad Legal

- **ROM Oficial Pinneada:**
  - `Fire Emblem (Europe)` (16 MB)
  - SHA-256: `6f2c30823ab0237310b138739ccfad879f1a07c4a782a9cbb92317d60d03cee9`
- **BIOS Oficial Pinneada:**
  - `gba_bios.bin` (16 KB)
  - SHA-256: `fd2547724b505f487e6dcb29ec2ecff3af35a841a77ab2e85fd87350abd36570`
- **Regla Inviolable:**
  - **NUNCA** versionar ni incluir archivos `.gba`, `.bin` ni assets propietarios del juego en el repositorio.
  - La aplicación en Android debe incluir un flujo inicial donde el usuario selecciona su propia copia de la ROM y BIOS en el almacenamiento de su teléfono (verificando los hashes antes de permitir jugar).

---

## 4. Reglas Operativas para el Agente

1. **Verificación antes de modificar:**
   - Mantener siempre la independencia entre este repositorio (`blazing-recomp`) y el repositorio del motor base (`gba-recomp`).
   - Todo código específico de Android, wrappers JNI, layouts táctiles o empaquetado debe residir dentro de este repositorio.
2. **Enfoque de Cobertura (*0 Interpreter*):**
   - Priorizar la cobertura completa de funciones de *Fire Emblem* mediante el mapeo de labels (`blazing.labels.toml`).
   - El intérprete de respaldo solo debe actuar como red de estabilidad durante el desarrollo.
3. **Calidad de Código y Estilo:**
   - Código Rust limpio, idiomático y seguro.
   - Preservar comentarios arquitectónicos y decisiones de diseño no obvias.
   - Crear commits atómicos con mensajes descriptivos siguiendo *Conventional Commits* (`feat:`, `fix:`, `refactor:`, `docs:`).

---

## 5. Notas y Reglas Personalizadas del Usuario

*(Espacio reservado para preferencias, decisiones de diseño y restricciones añadidas por el usuario durante el desarrollo)*

- 
