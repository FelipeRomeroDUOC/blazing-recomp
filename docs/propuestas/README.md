# Propuestas de Cambio (Nivel 2)

Directorio destinado a almacenar las propuestas de cambio de **Nivel 2** según el flujo de la skill [`propuesta-verificable`](file:///.agents/skills/propuesta-verificable/SKILL.md).

---

## Convención de nombres

Cada propuesta formal debe nombrarse siguiendo el formato:
```text
docs/propuestas/AAAA-MM-DD_nombre-descriptivo.md
```

## Estructura requerida

Las propuestas deben crearse a partir de la plantilla en [`.agents/skills/propuesta-verificable/assets/plantilla-propuesta.md`](file:///.agents/skills/propuesta-verificable/assets/plantilla-propuesta.md) y contener:
1. **Intención y Contexto:** Problema, restricciones y alternativas descartadas.
2. **Criterios de Aceptación:** Escenarios Dado / Cuando / Entonces con nombre de test.
3. **Suposiciones y Prototipos:** Identificación de riesgos antes de tocar código.
4. **Métodos de Comprobación:** Herramientas y comandos concretos para validar.
5. **Revisión Humana:** Puntos visibles para inspección y veredicto del usuario.
