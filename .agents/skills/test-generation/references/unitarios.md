# Tests unitarios

## Qué son
Prueban una función, método o clase **de forma aislada**. Si la unidad depende de algo externo (base de datos, red, archivos, reloj), ese algo se reemplaza por un doble de prueba.

## Dobles de prueba: cuál usar
- **Stub**: devuelve un valor fijo. Úsalo cuando solo necesitas que la dependencia responda algo.
- **Mock**: además verifica que fue llamado (con qué argumentos, cuántas veces). Úsalo cuando la llamada *es* el comportamiento que importa (por ejemplo, "se envió un correo").
- **Fake**: implementación simple y funcional (una base de datos en memoria). Úsalo cuando el stub se vuelve demasiado complicado.

Mockear de más es un error común: si todo está mockeado, el test solo prueba los mocks. Regla práctica: mockear lo que está en el borde del sistema (red, disco, reloj, servicios externos), no la lógica propia del proyecto.

## Qué casos escribir
Para cada unidad, cubrir como mínimo:
1. **Caso típico** (camino feliz).
2. **Límites**: el valor justo donde cambia el comportamiento (si el descuento aplica con total >= 100, probar 99, 100 y 101).
3. **Vacíos y nulos**: lista vacía, texto vacío, `None`/`null`, cero.
4. **Entradas inválidas**: tipo incorrecto, valores negativos, fuera de rango. Verificar la excepción exacta.
5. **Cada rama** de los `if`/`else`/`switch` relevantes.

Los límites son clave porque son justo lo que la mutación ataca (`>` cambiado por `>=`). Un test que no prueba el valor frontera deja sobrevivir ese mutante.

## Técnicas útiles
- **Tests parametrizados** (`pytest.mark.parametrize`, `test.each` en Jest, `@ParameterizedTest` en JUnit): una sola función de test con muchas entradas y salidas. Evitan copiar y pegar.
- **Tests basados en propiedades** (Hypothesis, fast-check): en lugar de ejemplos fijos, se declara una propiedad que siempre debe cumplirse (por ejemplo, "ordenar dos veces da lo mismo que ordenar una vez") y la herramienta genera cientos de entradas. Útiles para lógica pura.
- **Relojes y aleatoriedad inyectables**: si el código usa la hora actual o números aleatorios, pasar esas fuentes como parámetro o congelarlas en el test.

## Ejemplo (Python, pytest)

```python
import pytest
from tienda import calcular_total

@pytest.mark.parametrize("subtotal, esperado", [
    (99, 99),      # justo debajo del umbral: sin descuento
    (100, 90),     # en el umbral: 10% de descuento
    (101, 90.9),   # justo encima del umbral
    (0, 0),        # carrito vacío
])
def test_descuento_del_10_por_ciento_desde_100(subtotal, esperado):
    assert calcular_total(subtotal) == pytest.approx(esperado)

def test_subtotal_negativo_lanza_error():
    with pytest.raises(ValueError, match="negativo"):
        calcular_total(-1)
```

Fíjate en que los valores esperados están escritos a mano, no calculados con la fórmula del código.

## Señales de un mal test unitario
- Solo verifica que "no lanzó excepción".
- Repite la lógica de la función dentro del test.
- Falla si se renombra una variable privada o se reordena código interno.
- Necesita red, disco o una base de datos real.
- Depende de que otro test se haya ejecutado antes.
