"""
Ejemplo ilustrativo de tests para la propuesta de `ejemplo-propuesta.md`.

Está en Python/pytest solo porque el ejemplo usa ese proyecto: usa siempre el
framework y el estilo de tests que ya tenga el proyecto real. No copies estos
valores; copia el estilo.

Estado previo VERIFICADO ejecutando estos tests contra el proyecto de juguete,
antes de cambiar nada:
  - test_total_redondeado      -> falla por la razón correcta: devuelve
                                  35.699999999999996 y se esperaba 35.7
  - test_total_un_centavo      -> falla por la razón correcta: devuelve
                                  0.011899999999999999 y se esperaba 0.01
  - test_pantalla_sin_cambios  -> pasa (regresión: debe seguir pasando)
Después de aplicar el redondeo, los tres pasan.
"""

# --- tests/test_totales.py -------------------------------------------------
from app.totales import calcular_total


# Escenario: Total redondeado a 2 decimales   (tipo: normal)
# Un comportamiento por test; el nombre es el título del escenario.
# El valor esperado (35.7) sale del escenario aprobado, NO de ejecutar el
# código nuevo y copiar lo que devuelva.
def test_total_redondeado():
    items = [{"precio": 10, "cantidad": 3}]
    assert calcular_total(items) == 35.7


# Escenario: Total de un solo centavo   (tipo: límite)
# Valor muy pequeño, donde el resto del cálculo más se nota.
def test_total_un_centavo():
    items = [{"precio": 0.01, "cantidad": 1}]
    assert calcular_total(items) == 0.01


# --- tests/test_ui.py ------------------------------------------------------
from app.ui import mostrar


# Escenario: Pantalla sin cambios   (tipo: regresión)
# Protege la restricción "la pantalla mantiene su formato actual".
# Prueba lo observable (el texto que ve la persona), no qué función interna
# se llamó. Con un total exacto (119.0) no debería cambiar nada: si este test
# deja de pasar, el cambio rompió algo que existía.
def test_pantalla_sin_cambios():
    items = [{"precio": 100, "cantidad": 1}]
    assert mostrar(items) == "Total: 119.0 [Guardar]"
