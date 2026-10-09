# Tests de integración

## Qué son
Comprueban que **varias piezas funcionan bien juntas**: un servicio con su base de datos real, un endpoint HTTP con su capa de lógica, un productor con su cola de mensajes. Detectan errores que los unitarios no ven: consultas SQL incorrectas, serialización rota, configuración errónea, contratos que no coinciden entre módulos.

## Cuándo hacerlos
- El riesgo está en la **conexión** entre componentes y no en la lógica interna.
- Se usa SQL, un ORM, una API externa, un sistema de archivos o una cola.
- Hay migraciones, transacciones o permisos que pueden fallar.

No hace falta uno por cada función. Se escriben pocos y bien elegidos, centrados en los flujos importantes.

## Entorno de prueba
- **Base de datos**: usar la misma tecnología que producción cuando se pueda (contenedores con Testcontainers o docker compose). SQLite en memoria es aceptable solo si el código no usa funciones específicas de otro motor.
- **Servicios externos de terceros**: no llamar al servicio real. Usar un servidor simulado (WireMock, MSW, `responses`, `nock`) o el entorno sandbox oficial.
- **Configuración aparte**: variables de entorno y credenciales de prueba, nunca las de producción. Verificar que el test no pueda tocar datos reales.

## Aislamiento entre tests
Los tests de integración fallan de forma intermitente casi siempre por estado compartido. Para evitarlo:
- Cada test **prepara** sus propios datos y **limpia** al terminar (rollback de transacción, truncar tablas o base nueva por test).
- No depender del orden de ejecución.
- No reutilizar identificadores fijos que puedan chocar (usar datos únicos por test).
- Esperar con condiciones explícitas (reintentar hasta que algo ocurra, con límite), no con pausas fijas (`sleep`).

## Qué verificar
- Que el dato **realmente quedó guardado**: tras el `POST`, consultar la base y comprobar los campos, no solo el código de respuesta 201.
- Códigos de estado y cuerpo de respuesta en errores (400, 404, 409, 500 controlado).
- Transacciones: si un paso falla, no debe quedar un estado a medias.
- Restricciones: claves únicas, claves foráneas, campos obligatorios.
- Permisos y autenticación en los endpoints que lo requieran.

## Ejemplo (Python, pytest + API)

```python
def test_crear_usuario_lo_guarda_en_la_base(client, db):
    respuesta = client.post("/usuarios", json={"nombre": "Ana", "email": "ana@test.cl"})

    assert respuesta.status_code == 201
    guardado = db.execute("SELECT nombre, email FROM usuarios WHERE email = ?",
                          ("ana@test.cl",)).fetchone()
    assert guardado == ("Ana", "ana@test.cl")

def test_email_duplicado_devuelve_409_y_no_crea_otro(client, db):
    client.post("/usuarios", json={"nombre": "Ana", "email": "ana@test.cl"})
    respuesta = client.post("/usuarios", json={"nombre": "Otra", "email": "ana@test.cl"})

    assert respuesta.status_code == 409
    total = db.execute("SELECT COUNT(*) FROM usuarios").fetchone()[0]
    assert total == 1
```

## Errores comunes
- Verificar solo el código de estado y no el efecto real.
- Datos compartidos que hacen pasar o fallar tests según el orden.
- Llamar a servicios externos reales (lento, frágil, a veces con costo).
- Pruebas que tardan minutos y nadie las ejecuta. Si la suite es lenta, separarla (marcadores o carpetas) para ejecutar los unitarios con frecuencia y los de integración antes de subir cambios.
- Mockear tanto que deja de ser integración.
