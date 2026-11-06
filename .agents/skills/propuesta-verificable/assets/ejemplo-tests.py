"""
Ejemplo ilustrativo de tests para la propuesta de `ejemplo-propuesta.md`.

Está en Python/pytest solo porque el ejemplo usa ese proyecto: usa siempre el
framework y el estilo de tests que ya tenga el proyecto real. No copies estos
valores; copia el estilo.

Estado previo VERIFICADO ejecutando estos tests contra el proyecto de juguete,
antes de cambiar nada:
  - test_fecha_en_pantalla       -> falla por la razón correcta: la pantalla
                                    muestra "2026-11-20" y se esperaba "20/11/2026"
  - test_pedido_sin_fecha        -> falla por la razón correcta: la pantalla
                                    muestra "Entrega:  [Guardar]" y se esperaba
                                    "Entrega: sin fecha [Guardar]"
  - test_pantalla_mantiene_formato -> pasa (no debe cambiar: debe seguir pasando)
  - test_reporte_sin_cambios     -> pasa (no debe cambiar: debe seguir pasando)
Después de aplicar el cambio, los cuatro pasan.
"""

# --- tests/test_entregas.py ------------------------------------------------
from app.ui import mostrar_pedido
from app.reportes import linea_reporte


# C1 · Fecha en formato día/mes/año   (tipo: normal)
# Un comportamiento por test; el nombre es el título de la comprobación.
# El valor esperado ("20/11/2026") sale de la comprobación aprobada, NO de
# ejecutar el código nuevo y copiar lo que devuelva.
def test_fecha_en_pantalla():
    pedido = {"id": 1, "entrega": "2026-11-20"}
    assert mostrar_pedido(pedido) == "Entrega: 20/11/2026 [Guardar]"


# C2 · Pedido sin fecha   (tipo: extremo)
# El dato que falta es el caso extremo más probable en un pedido antiguo.
def test_pedido_sin_fecha():
    pedido = {"id": 2}
    assert mostrar_pedido(pedido) == "Entrega: sin fecha [Guardar]"


# C3 · La pantalla mantiene su formato   (tipo: no debe cambiar)
# Protege la regla R1. Prueba lo observable (el texto que ve la persona), no
# qué función interna se llamó. Si deja de pasar, el cambio rompió algo que
# ya existía.
def test_pantalla_mantiene_formato():
    texto = mostrar_pedido({"id": 3, "entrega": "2026-11-20"})
    assert texto.startswith("Entrega: ") and texto.endswith(" [Guardar]")


# C4 · El reporte sigue igual   (tipo: no debe cambiar)
# Protege la regla R2.
def test_reporte_sin_cambios():
    assert linea_reporte({"id": 7, "entrega": "2026-11-20"}) == "7;2026-11-20"
