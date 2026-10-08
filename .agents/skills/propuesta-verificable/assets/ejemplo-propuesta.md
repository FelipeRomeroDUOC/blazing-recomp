# Ejemplo de propuesta de nivel 2 (ilustrativo)

Nota: ejemplo inventado, solo para calibrar cuánta concreción se espera. No copies su contenido ni sus cifras; usa el estilo.

## Frases vagas → versión verificable

| Vaga | Verificable |
|------|-------------|
| "Mejora el cálculo del total" | "`calcular_total` devuelve el total redondeado a 2 decimales (hoy devuelve 35.699999999999996 para 3 unidades de 10)." |
| "Podría afectar a otras partes" | "Lo usan `app/ui.py` línea 4 y `app/reportes.py` línea 3 (búsqueda: `grep -rn calcular_total app`)." |
| "Se probará a fondo" | "`pytest tests/test_totales.py::test_total_redondeado`; pasa si devuelve 35.7; no detecta cómo se ve el total en pantalla." |
| "Se cubrirán los casos importantes" | "Normal: 3 unidades de 10. Límite: un ítem de 0.01. Regresión: la pantalla para 100 × 1 sigue mostrando 'Total: 119.0 [Guardar]'." |
| "Respeta las convenciones" | "La pantalla mantiene su formato 'Total: X [Guardar]' (origen: respuesta humana, 2026-10-05) → implica: no cambiar `ui.py` → protegida por: escenario 'Pantalla sin cambios'." |
| "Debería funcionar con la librería" | "Suposición: `round(x, 2)` da el mismo resultado con enteros y decimales → prototipar: sí." |

## Propuesta completa (resumida)

# Propuesta: redondear total
Fecha: 2026-10-05 | Nivel: 2 (lógica compartida) | Estado: pendiente

### 1. Qué cambia
El total de un pedido se muestra y se acumula en reportes con 2 decimales, sin restos de cálculo.

### 2. Contexto
- Contexto consultado: `app/totales.py`, `app/ui.py`, `app/reportes.py`, `CONTEXTO.md`.
- Intención: que el total mostrado y el sumado en reportes coincidan al centavo.
- Restricciones y decisiones previas:
  - La pantalla mantiene su formato actual "Total: X [Guardar]" (origen: respuesta humana, 2026-10-05) → implica: no tocar `ui.py` → protegida por: "Pantalla sin cambios".
  - `reportes.py` ya fue ajustado en una propuesta anterior (origen: `propuestas/2026-10-03_reportes.md`) → implica: no cambiar su estructura → protegida por: no testeable.
- Alternativas descartadas:
  - Redondear en `ui.py` → descartada porque `reportes.py` seguiría sumando valores sin redondear.
- Ante imprevistos, prioriza: no alterar resultados ya guardados antes que simplificar el código.

### 3. Ejemplos
- Normal: 3 unidades de 10 → antes: 35.699999999999996 → ahora: 35.7
- Límite: un ítem de 0.01 → antes: 0.011899999999999999 → ahora: 0.01

### 6. Suposiciones
- `round(x, 2)` da el mismo resultado con enteros y decimales → prototipar: sí
  → si es falso: convertir a `Decimal` antes de redondear
  → resultado del prototipo: (se completa tras la aprobación) | no detecta: otros intérpretes o versiones
- Ningún otro módulo compara el total con igualdad exacta → prototipar: no
  → si es falso: habría que revisar esas comparaciones en el mismo cambio

### 8. Criterios de aceptación
Escenario: Total redondeado a 2 decimales
Caso de uso: el cliente ve el total de su pedido | Tipo: normal
  Dado que hay 3 unidades con precio 10
  Cuando calculo el total
  Entonces obtengo 35.7
Test: tests/test_totales.py::test_total_redondeado (se escribe tras la aprobación)
Estado previo: (se completa al escribir el test)

Escenario: Total de un solo centavo
Caso de uso: el cliente compra un artículo de muy bajo precio | Tipo: límite
  Dado que hay 1 unidad con precio 0.01
  Cuando calculo el total
  Entonces obtengo 0.01
Test: tests/test_totales.py::test_total_un_centavo (se escribe tras la aprobación)
Estado previo: (se completa al escribir el test)

Escenario: Pantalla sin cambios
Caso de uso: el cliente ve el total en pantalla | Tipo: regresión
  Dado que hay 1 unidad con precio 100
  Cuando se muestra el pedido
  Entonces la pantalla dice "Total: 119.0 [Guardar]"
Test: tests/test_ui.py::test_pantalla_sin_cambios (se escribe tras la aprobación)
Estado previo: (se completa al escribir el test; debe pasar)

Los tests de este ejemplo, ya escritos, están en `assets/ejemplo-tests.py`.

### 9. Cómo lo comprobaré
- Total redondeado → `pytest tests/test_totales.py::test_total_redondeado` → pasa si devuelve 35.7 → no detecta cómo se ve en pantalla
- Total de un solo centavo → `pytest tests/test_totales.py::test_total_un_centavo` → pasa si devuelve 0.01 → no detecta otros montos pequeños
- Pantalla sin cambios → `pytest tests/test_ui.py::test_pantalla_sin_cambios` → pasa si el texto es "Total: 119.0 [Guardar]" → no detecta el aspecto visual de la pantalla

## Revisión humana
- Restricción "no cambiar la estructura de `reportes.py`" (no testeable): ✔ | ✘ | ?
