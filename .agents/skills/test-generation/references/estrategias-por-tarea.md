# Estrategias según la tarea: antes y después de editar

## Qué son
La efectividad de un test no depende solo de qué prueba, sino de **cuándo se escribe**. Escribir tests antes de tocar el código previene sesgos de confirmación, fuerza a entender el contrato y demuestra que el test realmente tiene la capacidad de detectar el error. Escribir tests después amplía la cobertura a casos frontera y asegura robustez.

Esta estrategia aplica a cualquier lenguaje y framework: la lógica y la secuencia temporal son universales.

## Cuándo escribir cada tipo de test

| Tipo de tarea | Test previo (ANTES de editar) | Condición previa | Test posterior (DESPUÉS de editar) |
|---|---|:---:|---|
| **Bugfix** | Reproducción del fallo | **Debe FALLAR** | Casos frontera del arreglo |
| **Refactor** | Caracterización / Regresión | **Debe PASAR** | Ninguno nuevo (mantener suite verde) |
| **Nueva feature** | Contrato / Especificación (TDD) | **Debe FALLAR** | Casos límite y estrés |

---

## 1. Al corregir un bug (Test de reproducción)

### Regla innegociable
Antes de cambiar cualquier línea en el código de producción, escribir un test que reproduzca exactamente el fallo reportado o descubierto.
- **El test DEBE fallar en rojo.** Si pasa antes de modificar la implementación, el test no está aislando el bug real o el diagnóstico es erróneo.
- **Después de corregir:** Debe pasar en verde sin alterar la aserción ni debilitar la prueba.

### Ejemplo (Python, pytest)
Supongamos que usuarios con saldo exacto son rechazados por un operador incorrecto (`saldo > monto` en vez de `>=`).

```python
# 1. Escribir ANTES del arreglo: debe FALLAR (rojo)
def test_transferencia_con_saldo_exacto_debe_completarse():
    cuenta = Cuenta(saldo=100)
    resultado = cuenta.transferir(monto=100, destino="otra_cuenta")

    assert resultado.exitosa is True
    assert cuenta.saldo == 0
```
> *Si este test pasa antes de tocar el código de producción, no toques la función: la hipótesis del fallo es incorrecta o el test no reproduce el caso.*

---

## 2. Al refactorizar (Tests de caracterización / regresión)

### Regla innegociable
Refactorizar es alterar la estructura interna sin cambiar el comportamiento observable.
- Antes de modificar el código, identificar o escribir tests que "congelen" las salidas conocidas para entradas representativas (camino feliz, errores esperados y valores borde).
- **Los tests DEBEN pasar en verde antes de refactorizar.** Son la red de seguridad de no-regresión.
- **Después del refactor:** Todos los tests preexistentes deben seguir pasando sin haber modificado sus asserts.

### Ejemplo (Python, pytest)
Antes de reescribir un parser manual o una rutina compleja:

```python
# Congelar comportamiento observable antes de refactorizar
import pytest

@pytest.mark.parametrize("entrada, esperado", [
    ("clave: valor", {"clave": "valor"}),
    ("clave:  espacios  ", {"clave": "espacios"}),
    ("invalido_sin_dos_puntos", {}),
])
def test_caracterizacion_parser_actual(entrada, esperado):
    assert parsear_encabezado(entrada) == esperado
```

---

## 3. Al implementar una nueva funcionalidad (Tests de contrato)

### Regla innegociable
Establecer las expectativas de entrada, salida y efectos secundarios antes o en paralelo a la implementación (estilo TDD).
- Diseñar la interfaz tal como le gustaría al consumidor usarla.
- El test fallará hasta que la función o módulo exista.

### Ejemplo (Python, pytest)

```python
def test_nuevo_calculador_de_descuento_aplica_regla_por_categoria():
    calculador = CalculadorDescuento(categoria="VIP")
    precio_final = calculador.aplicar(precio_base=100)

    assert precio_final == 80
```

---

## 4. Al finalizar cualquier implementación (Casos límite y mutación)

### Regla innegociable
Una vez que la funcionalidad básica o el fix está en verde, blindar el código con casos adversos:
- **Valores frontera:** listas vacías, cadenas vacías, `None`/`null`, ceros, valores negativos, números máximos permitidos.
- **Comprobación de solidez:** Cambiar deliberadamente una condición lógica en el código (`>` por `>=`, `==` por `!=`) y verificar que la suite falle de inmediato. Si no falla, los asserts son insuficientes.

---

## Errores comunes en la estrategia temporal
- **Arreglar el bug primero y escribir el test después:** Se tiende a escribir un test que solo valida lo que el código ya hace (sesgo de confirmación), dejando pasar variantes del mismo error.
- **Dar por bueno un test de bugfix sin verlo fallar:** Si no se vio el fallo en rojo, no hay prueba de que el test sirva para algo.
- **Modificar los tests existentes durante un refactor:** Si un test existente falla al refactorizar, se introdujo una regresión; no se debe alterar el test para "forzar el verde".
- **Omitir casos límite por tener el camino feliz en verde:** La mayoría de fallos en producción ocurren en valores nulos, vacíos o extremos no probados.
