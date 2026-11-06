# Ejemplo de propuesta de nivel 2 (ilustrativo)

Nota: ejemplo inventado, solo para calibrar longitud y estilo. No copies su contenido ni sus datos. Los tests de este ejemplo están en `assets/ejemplo-tests.py`.

---

# Propuesta: fecha de entrega legible

Fecha: 2026-11-05 · Nivel 2 · Estado: pendiente
Viene de: no corrige ninguna
Hereda sin cambios: no aplica

## Para decidir

**Qué cambia:** la pantalla de pedido muestra la fecha de entrega como 20/11/2026 en vez de 2026-11-20.

**Para qué:** que la persona que atiende el mostrador lea la fecha sin tener que darle la vuelta.

**Ejemplo:** pedido con entrega el 20 de noviembre → hoy: "Entrega: 2026-11-20 [Guardar]" → después: "Entrega: 20/11/2026 [Guardar]".

**Qué voy a hacer:**
1. Agregar una función nueva que convierte "2026-11-20" en "20/11/2026".
2. Hacer que la pantalla de pedido use esa función.
3. Dejar los reportes como están.

**Qué podría romperse:** nada que ya funcione, porque no cambio la función que hoy usan la pantalla y el reporte (ver E1). Lo que sí podría pasar es que un pedido guardado con otro formato de fecha dé error (ver S1).

**Qué no voy a tocar:** los reportes y el texto "Entrega: … [Guardar]".

**Necesito de ti:**
1. Si un pedido no tiene fecha, ¿qué debe verse?
   Opciones: "sin fecha" / dejar en blanco / no sé
   Si no respondes, uso: "sin fecha"
   Cambia el plan si: eliges en blanco, el paso 1 devuelve texto vacío en ese caso.

Tendrás que mirar tú: que la fecha cabe en la pantalla sin cortarse (no se puede comprobar con código).

---

## Detalle para verificar

### Suposiciones (lo que doy por cierto sin haberlo comprobado)

S1 · Todas las fechas guardadas están escritas como AAAA-MM-DD.
  Si es falso: la función nueva debe aceptar también el otro formato.
  Se prueba antes de editar: sí (si es falso, cambia el paso 1; se prueba leyendo los datos de ejemplo del proyecto)
  Resultado de la prueba: se completa tras la aprobación

S2 · Ningún otro código compara la fecha como texto.
  Si es falso: habría que revisar esas comparaciones en el mismo cambio.
  Se prueba antes de editar: no

### Reglas del proyecto que respeto (máximo 3)

R1 · La pantalla siempre muestra "Entrega: X [Guardar]".
  Fuente: CONTEXTO.md
  Por eso: solo cambio X, no el texto de alrededor.
  Se comprueba con: C3

R2 · Los reportes siguen mostrando AAAA-MM-DD.
  Fuente: respuesta humana, 2026-11-02 (la contadora los importa así)
  Por eso: no toco la función que usa el reporte.
  Se comprueba con: C4

### Comprobaciones (criterios de aceptación; no se cambian sin aprobación)

C1 · Fecha en formato día/mes/año — normal
  Situación real: la persona del mostrador abre un pedido.
  Dado que el pedido tiene entrega "2026-11-20"
  Cuando se muestra el pedido
  Entonces la pantalla dice "Entrega: 20/11/2026 [Guardar]"
  Test: tests/test_entregas.py::test_fecha_en_pantalla · Estado previo: falla
  Se ejecuta con: `python3 -m pytest tests/test_entregas.py::test_fecha_en_pantalla`
  No cubre: cómo se ve en pantalla

C2 · Pedido sin fecha — extremo
  Situación real: se abre un pedido antiguo que no guardó fecha.
  Dado que el pedido no tiene fecha de entrega
  Cuando se muestra el pedido
  Entonces la pantalla dice "Entrega: sin fecha [Guardar]"
  Test: tests/test_entregas.py::test_pedido_sin_fecha · Estado previo: falla
  Se ejecuta con: `python3 -m pytest tests/test_entregas.py::test_pedido_sin_fecha`
  No cubre: fechas con formato inválido

C3 · La pantalla mantiene su formato — no debe cambiar
  Situación real: la persona sigue viendo el mismo texto alrededor de la fecha.
  Dado que el pedido tiene entrega "2026-11-20"
  Cuando se muestra el pedido
  Entonces el texto empieza con "Entrega: " y termina con " [Guardar]"
  Test: tests/test_entregas.py::test_pantalla_mantiene_formato · Estado previo: pasa
  Se ejecuta con: `python3 -m pytest tests/test_entregas.py::test_pantalla_mantiene_formato`
  No cubre: tamaño ni posición en pantalla

C4 · El reporte sigue igual — no debe cambiar
  Situación real: la contadora importa el reporte del día.
  Dado que el pedido 7 tiene entrega "2026-11-20"
  Cuando se genera su línea de reporte
  Entonces la línea es "7;2026-11-20"
  Test: tests/test_entregas.py::test_reporte_sin_cambios · Estado previo: pasa
  Se ejecuta con: `python3 -m pytest tests/test_entregas.py::test_reporte_sin_cambios`
  No cubre: la importación en sí

### Archivos

Tocaré: app/entregas.py, app/ui.py
Podrían verse afectados: app/reportes.py (no lo toco; ver E1)

### Evidencia

E1 · La función actual la usan la pantalla y el reporte — origen: `grep -rn formatear_fecha app` → app/ui.py:5 y app/reportes.py:5

### Decisiones de diseño

Alternativa descartada: cambiar la función actual — porque el reporte dejaría de salir como AAAA-MM-DD (R2).
Ante imprevistos, prioriza: no alterar el reporte antes que reutilizar código.

### Antes de presentar, corregí

Mi primer borrador cambiaba la función actual; al buscar referencias vi que el reporte la usa (E1) y pasé a una función nueva.

---

## Revisión humana

- La fecha cabe en la pantalla sin cortarse: ✔ | ✘ | ?
