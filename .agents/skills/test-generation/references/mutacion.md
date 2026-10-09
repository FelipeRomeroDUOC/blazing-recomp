# Tests de mutación

## Qué es (y qué no es)
La mutación **no genera tests**: mide qué tan buenos son los que ya existen. Una herramienta modifica el código en pequeños detalles (cada cambio es un *mutante*) y ejecuta la suite:

- Si algún test **falla**, el mutante fue **eliminado** (killed). Bien.
- Si todos los tests **pasan**, el mutante **sobrevivió** (survived). Mal: hay un comportamiento que ningún test vigila.

El **mutation score** es el porcentaje de mutantes eliminados. Es más honesto que la cobertura de líneas, porque una línea puede ejecutarse sin que ningún test verifique su resultado.

## Mutaciones típicas
- Operadores relacionales: `>` por `>=`, `==` por `!=`.
- Operadores aritméticos: `+` por `-`, `*` por `/`.
- Lógicos: `and` por `or`, negar condiciones.
- Constantes: `0` por `1`, `True` por `False`.
- Eliminar una llamada o un `return`.
- Cambiar un valor de retorno por `None`/`null`.

## Herramientas según lenguaje
| Lenguaje | Herramienta |
|---|---|
| JavaScript / TypeScript / C# | Stryker |
| Java / Kotlin | PIT (Pitest) |
| Python | mutmut, cosmic-ray |
| Rust | cargo-mutants |
| Go | go-mutesting, gremlins |
| PHP | Infection |

Antes de instalar algo, revisar si el proyecto ya tiene una configurada, y avisar al usuario si hay que agregar una dependencia.

## Flujo para el agente
1. **Línea base**: ejecutar la suite normal y confirmar que está en verde. La mutación sobre tests que ya fallan no sirve.
2. **Acotar el alcance**: la mutación es lenta (cada mutante implica ejecutar tests). Limitarla a los archivos o carpetas que se están trabajando, no a todo el proyecto.
3. **Ejecutar** la herramienta y leer el reporte de supervivientes.
4. **Clasificar cada superviviente**:
   - *Falta un test*: el comportamiento es real y nadie lo verifica. Agregar un test que lo mate.
   - *Test débil*: existe un test pero su assert es vago. Reforzar el assert.
   - *Mutante equivalente*: el cambio no altera el comportamiento observable (por ejemplo, cambiar `i < n` por `i != n` en un bucle que siempre termina igual). No se puede matar; ignorarlo y anotarlo.
   - *Código muerto o inútil*: si ningún test podría detectar el cambio porque el código no sirve, señalarlo al usuario como candidato a eliminar.
5. **Repetir** sobre los supervivientes hasta que queden solo los equivalentes o se alcance un umbral razonable.
6. **Reportar**: mutation score antes y después, qué tests se agregaron, qué mutantes se descartaron y por qué.

## Cómo matar un mutante: ejemplo
Código:
```python
def es_mayor_de_edad(edad):
    return edad >= 18
```
Mutante sobreviviente: `edad > 18`. Esto indica que no se probó el valor frontera. Test que lo mata:
```python
def test_18_anios_ya_es_mayor_de_edad():
    assert es_mayor_de_edad(18) is True
```

## Qué umbral esperar
No perseguir 100%: los mutantes equivalentes lo impiden y el esfuerzo se vuelve desproporcionado. Como orientación, entre 70% y 85% suele ser una buena meta en código con lógica importante. Priorizar módulos críticos (cálculos, reglas de negocio, validaciones) sobre código de glue o de configuración.

## Reglas para el agente
- **Nunca cambiar el código de producción** para eliminar mutantes. Solo se mejoran los tests.
- **No excluir mutantes** en la configuración solo para subir el puntaje. Si se excluye algo, justificarlo.
- Los tests nuevos deben tener sentido por sí mismos (verificar un comportamiento real), no ser un parche para un mutante específico.
- Avisar al usuario si la ejecución va a tardar, e indicar el comando exacto usado para que pueda repetirla.
