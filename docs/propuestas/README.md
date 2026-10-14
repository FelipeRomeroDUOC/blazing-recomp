# Propuestas de Cambio (Nivel 2)

Directorio destinado a almacenar las propuestas de cambio de **Nivel 2** según el flujo de la skill [`propuesta-verificable`](../../.agents/skills/propuesta-verificable/SKILL.md).

---

## Convención de nombres

Cada propuesta formal debe nombrarse siguiendo el formato:
```text
docs/propuestas/AAAA-MM-DD_nombre-descriptivo.md
```

## Metadato obligatorio

Las tres primeras líneas de una propuesta son su ficha, y la tercera es la que los scripts y los
agentes leen para saber en qué punto está:

```text
Fecha: AAAA-MM-DD
Nivel: 2
Estado: pendiente
```

`Estado` es una **línea propia** (no va dentro de negritas ni de una tabla) y su valor sale de este
vocabulario cerrado. Un test lo comprueba:
[`tests/packaging_tests.rs::test_propuestas_declaran_estado_valido`](../../tests/packaging_tests.rs).

| Estado | Significa |
| :--- | :--- |
| `pendiente` | Escrita y presentada; **nadie la ha aprobado** todavía. Prohibido editar código. |
| `aprobada` | El usuario aprobó el alcance. Puede llevar nota: `aprobada (el usuario eligió X el AAAA-MM-DD)`. |
| `verificada por agente` | Ejecutada y comprobada con los escenarios que se pueden correr sin el dispositivo. |
| `revisada por humano` | Además, una persona revisó lo que exige ojos (el televisor, la ergonomía). |
| `modificada` | Aprobada, pero el alcance cambió a mitad y quedó registrado en «Cambios tras prototipos». |
| `descartada` | Aprobada y luego abandonada sin llegar a medir nada concluyente. |
| `discrepada` | Ejecutada y **la medida contradijo la premisa**. Lleva su sección `## Discrepancia`. |
| `cerrada` | Terminada y con su `Resultado` completo; no queda nada por hacer en ella. |

Reglas que este vocabulario sostiene:

- Una propuesta **no se cierra sin `Resultado`**: predicho vs. real, y qué se respetó del contexto.
- Un estado `verificada por agente` **no** implica verificado en el dispositivo: eso es
  `revisada por humano`. Las que esperan el televisor están listadas en
  [`docs/BACKLOG.md`](../BACKLOG.md#esperando-el-televisor-28-propuestas-con-revisión-humana-pendiente).
- Un hallazgo que se descubre y no se arregla **no se borra**: se anota en `### Fuera de alcance` y
  entra en el registro de [`docs/BACKLOG.md`](../BACKLOG.md#registro-de-hallazgos), que audita
  [`scripts/auditar_hallazgos.py`](../../scripts/auditar_hallazgos.py).

> **Nota histórica.** Dos propuestas del 2026-10-11 se escribieron con una plantilla anterior, que
> ponía la ficha en negritas (`**Estado:** Implementada y Verificada`) y no incluía «Criterios de
> aceptación». No se reescriben —son registro de lo que se midió— pero llevan ya su línea `Estado:`
> canónica al lado de la histórica. Lo mismo vale para cualquier propuesta anterior al 2026-10-12:
> **el registro histórico no se retoca**, se normaliza su ficha.

## Estructura requerida

Las propuestas deben crearse a partir de la plantilla en [`.agents/skills/propuesta-verificable/assets/plantilla-propuesta.md`](../../.agents/skills/propuesta-verificable/assets/plantilla-propuesta.md) y contener:
1. **Intención y Contexto:** Problema, restricciones y alternativas descartadas.
2. **Criterios de Aceptación:** Escenarios Dado / Cuando / Entonces con nombre de test.
3. **Suposiciones y Prototipos:** Identificación de riesgos antes de tocar código.
4. **Métodos de Comprobación:** Herramientas y comandos concretos para validar.
5. **Revisión Humana:** Puntos visibles para inspección y veredicto del usuario.
