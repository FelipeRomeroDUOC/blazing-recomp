# Propuesta: Contar los píxeles opacos que aporta cada sprite afín de 7500, en la PPU y en el shim

Fecha: 2026-10-18
Nivel: 2
Estado: verificada por agente

<!-- Estados: pendiente | aprobada | modificada | descartada |
     verificada por agente | revisada por humano | discrepada | cerrada -->

## Propuesta (la escribe el agente antes de editar)

### 1. Qué cambia

Se crea un ejemplo nuevo de solo lectura en `blazing-recomp/examples/shim_afin_pixeles.rs`, con su entrada en `Cargo.toml`. No cambia `src/`, ni `gba-core`, ni los ejemplos existentes.

Para cada uno de los 5 sprites afines del cuadro 7500:

- Se aísla ese sprite: en una copia de la OAM se desactivan los demás (se marcan como deshabilitados en `attr0`). Se mide la PPU de referencia con esa OAM aislada y se cuentan los píxeles opacos que dibuja el sprite.
- Se hace lo mismo con el shim completo sobre esa OAM aislada.
- Se mide la PPU sin ningún afín (todos los afines desactivados) como línea base.

Salida: `volcados/afin-pixeles-7500.txt`, con una fila por sprite: píxeles opacos en la PPU, píxeles opacos en el shim, y el número de píxeles en que la PPU y el shim difieren dentro de la caja del sprite.

### 2. Contexto

- Contexto consultado:
  - `docs/propuestas/2026-10-18_referencia-afines-gba-core.md` (verificada): la PPU aplica pa, pb, pc, pd (`ppu.rs`, `render_sprites`, líneas ~420 a 462), así que sirve como referencia para afines.
  - `docs/propuestas/2026-10-18_afines-conteo-matriz-7500.md` (verificada): en 7500 hay 5 afines visibles con matriz no identidad, y shim y PPU coinciden en todos los píxeles. Eso es contradictorio y hay que aclararlo.
  - `src/ppu_shim.rs`, `decode_oam`: los afines se tratan como sprites normales, sin matriz (según lectura previa).
- Intención: saber si los 5 afines aportan algún píxel en pantalla, y si el shim los dibuja.
- Hipótesis:
  - H210: los 5 afines no aportan ningún píxel opaco en la PPU (tiles vacíos o fuera de la textura). Si se confirma, el 0 de diferencias es vacío y no dice nada sobre el shim.
  - H211: la PPU aporta píxeles opacos, pero el shim aporta 0 en esos sprites. Si se confirma, el shim no dibuja los afines y la comparación del cuadro completo debía dar diferencias; habría que revisar por qué no las dio.
  - H212: la PPU y el shim aportan el mismo número de píxeles opacos en cada afín. Si se confirma, el shim dibuja los afines (aunque sea sin matriz) y el 0 de diferencias en el cuadro completo se explica por esa coincidencia.
- Restricciones y decisiones previas:
  - Solo el cuadro 7500, con `secuencias\intro.txt`.
  - Sin cambios en el shim ni en `gba-core`.
  - Tolerancia ±1 por canal, igual que en las medidas anteriores.
- Alternativas descartadas:
  - Implementar la transformación afín en el shim ahora → descartada: primero hay que saber si los afines se ven en la PPU y en el shim.

### 3. Ejemplos

- Caso normal: el afín 2 tiene 96 píxeles opacos en la PPU y 0 en el shim → H211.
- Caso límite: el afín 4 tiene 0 píxeles opacos en la PPU → H210 para ese sprite, sin importar el shim.

### 4. Pasos

1. Crear `examples/shim_afin_pixeles.rs` (`required-features = ["proto"]`), reutilizando la captura por línea y la función de renderizado del shim de `shim_afin_mide.rs`.
2. Para cada afín (índices 0 a 4) y para la línea base: preparar la OAM aislada, renderizar la PPU línea a línea y el shim completo, y contar píxeles opacos y diferencias.
3. Controles: la línea base (sin afines) debe dar 0 píxeles opacos de afines en la PPU; la PPU con todos los afines debe coincidir con el framebuffer del emulador (0 diferencias), como antes.
4. Escribir `volcados/afin-pixeles-7500.txt`.
5. Decidir H210, H211 y H212 y registrar el Resultado.

### 5. Qué más podría verse afectado

- Archivos que cambian: `examples/shim_afin_pixeles.rs` (nuevo), `Cargo.toml` (una entrada), `volcados/afin-pixeles-7500.txt` (nuevo).
- Archivos que no cambian: `src/*`, `gba-core`, ejemplos existentes, pruebas, propuestas anteriores.
- Riesgo: al aislar un sprite, el cambio de OAM puede alterar el estado de la línea (por ejemplo, la prioridad). Se mide la línea base para controlarlo.

### 6. Suposiciones

- Desactivar un sprite con `attr0` bit 9 (sin afín) lo quita de la PPU y del shim sin otros efectos → prototipar: la línea base debe dar 0 píxeles de afines → si es falsa: parar.
- La captura por línea de la intro contiene la OAM correcta del cuadro 7500 → prototipar: el control con framebuffer del emulador debe dar 0 diferencias → si es falsa: parar.

### 7. Alcance

- Incluye: los 5 afines de 7500; PPU y shim; línea base; H210 a H212; resultado.
- No incluye: implementar afines; otros cuadros; cambios en el shim o en `gba-core`.

### 8. Criterios de aceptación (no se modifican sin aprobación)

Escenario: se cuentan los píxeles de cada afín
Caso de uso: decidir si los afines aportan píxeles | Tipo: normal
  Dado el cuadro 7500
  Cuando se corre el ejemplo
  Entonces existe `volcados/afin-pixeles-7500.txt` con una fila por afín (0 a 4) y su conteo en PPU y shim
Test: no se escribe prueba nueva; se revisa el informe
Estado previo: falla (el ejemplo no existe)

Escenario: la línea base no tiene píxeles de afines
Caso de uso: que el aislamiento sea válido | Tipo: límite
  Dado la OAM con todos los afines desactivados
  Cuando se renderiza la PPU
  Entonces el conteo de píxeles de afines es 0
Test: aserción en el ejemplo; el informe lo muestra
Estado previo: falla

### 9. Cómo lo comprobaré

- Ejecución: `cargo run --release --features proto --example shim_afin_pixeles -- "roms\1575 - Fire Emblem (E)(Independent).gba" secuencias\intro.txt 7500`, lanzado como job con salida a fichero. Pasa si existe el informe y los controles dan lo esperado.
- Revisión manual de la tabla por afín.

### 10. Terminado cuando

Existe el informe y el Resultado dice si H210, H211 y H212 se confirman o descartan para cada afín.

### 11. Parar si

- La línea base da píxeles de afines: parar y corregir el aislamiento antes de medir.
- El control con framebuffer no da 0: parar, porque la referencia no es válida.
- La lectura de archivos o la herramienta falla dos veces seguidas: parar y registrarlo en el Resultado.

---

## Resultado (lo completa el agente después de editar)

Aprobada por el usuario ("Aprobado"). Ejecutado: `examples/shim_afin_pixeles.rs` (nuevo), entrada en `Cargo.toml`, informe `volcados/afin-pixeles-7500.txt`. Sin cambios en `src/` ni en `gba-core`.

Método: cada afiné queda activo solo en su variante de OAM; los demás afines pasan a forma prohibida (attr0 bits 14-15 = 3). Los píxeles que aporta un afín son los que cambian frente a la línea base (todos los afines apagados).

Controles:
- PPU con OAM completa frente al framebuffer del emulador: 0 diferencias.
- Línea base frente al emulador: 0 diferencias (los afines no cambian ningún píxel en la PPU).

Resultados por afiné (0 a 4, todos 32x64 doble tamaño, y = 24):
- PPU aporta 0 píxeles en los cinco.
- Shim aporta 0 píxeles en los cinco.
- Diferencias PPU-shim en esos píxeles: 0 (no hay píxeles que comparar).

Decisión:
- H210 (la PPU no aporta píxeles opacos): **confirmada** para los cinco.
- H211 (la PPU aporta y el shim no): descartada.
- H212 (ambos aportan igual): descartada.

Lectura: el 0 de diferencias de la medición anterior se explica porque los afines no dibujan nada en este cuadro, en la PPU ni en el shim. Por tanto, en 7500 no hay evidencia de que el shim dibuje mal los afines, pero tampoco de que los dibuje bien: no hay nada que dibujar.

Limitación: el método cuenta cambios de píxel. Un sprite que pinte el mismo color que el fondo no aparecería en el conteo. No se ha comprobado si los tiles de estos afines están vacíos o si la PPU los oculta por otra razón (ventana, prioridad, tile base).

Próximo paso propuesto (sin aprobar): comprobar por qué estos cinco afines no aportan píxeles en la PPU: leer sus tiles (contar píxeles no transparentes en la VRAM de cada tile) y su tile base. Requiere propuesta y aprobación.

---

## Revisión humana

- Que el siguiente paso sobre afines (implementar o no la transformación) se decida con estos resultados: ✔ | ✘ | ?
