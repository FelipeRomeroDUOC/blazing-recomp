//! Contrato de la línea de segundo (`t=`) de la telemetría de rendimiento.
//!
//! El contrato vive en [`ESQUEMA_TEXTO`] (`telemetria/esquema.txt`), un fichero de
//! texto plano que es el **dueño único** de la lista de campos: lo lee este módulo
//! para escribir y `scripts/diagnostico/analizar_telemetria.py` para leer. Añadir
//! un campo se hace en el esquema, y los dos lados lo entienden sin tocar código.
//!
//! Antes de este módulo el contrato estaba escrito cuatro veces (el `format!` de
//! `PerfReport::to_line`, el regex posicional de Python, la tabla de la guía y su
//! línea de ejemplo) y la deriva entre ellos fallaba en silencio: el regex no casaba,
//! el segundo no se contaba y el informe salía con medias sesgadas sin avisar.
//!
//! El esquema se analiza una vez (`OnceLock`) y un esquema mal formado es un `Err`,
//! nunca un pánico: un contrato roto tiene que verse, no tumbar el juego.

use std::collections::BTreeMap;
use std::sync::OnceLock;

/// Texto del esquema, embebido en la biblioteca en tiempo de compilación.
///
/// Va embebido (`include_str!`) y no como asset: el APK no lo lleva como archivo
/// y el módulo no necesita ni sistema de ficheros ni dependencias nuevas.
pub const ESQUEMA_TEXTO: &str = include_str!("../telemetria/esquema.txt");

/// Cómo se imprime un campo de la línea de segundo.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Tipo {
    /// Entero sin sufijo (`4945`).
    Entero,
    /// Entero con el sufijo literal `s` (`45s`). El sufijo es del texto, no de la unidad.
    EnteroS,
    /// Real con un decimal (`60.0`).
    Real,
    /// Par promedio/máximo de reales (`13.6/21.9`).
    RealReal,
    /// Par `ancho x alto` de enteros (`1280x720`).
    EnteroEntero,
}

impl Tipo {
    /// Nombre del tipo tal como se declara en el esquema.
    pub fn nombre(&self) -> &'static str {
        match self {
            Tipo::Entero => "entero",
            Tipo::EnteroS => "entero_s",
            Tipo::Real => "real",
            Tipo::RealReal => "real/real",
            Tipo::EnteroEntero => "entero_x_entero",
        }
    }

    /// Interpreta el nombre de un tipo del esquema.
    pub fn desde_texto(texto: &str) -> Option<Tipo> {
        match texto {
            "entero" => Some(Tipo::Entero),
            "entero_s" => Some(Tipo::EnteroS),
            "real" => Some(Tipo::Real),
            "real/real" => Some(Tipo::RealReal),
            "entero_x_entero" => Some(Tipo::EnteroEntero),
            _ => None,
        }
    }

    /// Imprime un valor con la forma de este tipo, o falla si el valor no encaja.
    ///
    /// El mensaje nombra el campo: `el valor 'fps' no encaja con el tipo Real`.
    fn formatear(&self, nombre: &str, valor: &Valor) -> Result<String, String> {
        match (self, valor) {
            (Tipo::Entero, Valor::Entero(n)) => Ok(format!("{n}")),
            (Tipo::EnteroS, Valor::Entero(n)) => Ok(format!("{n}s")),
            (Tipo::Real, Valor::Real(v)) => Ok(format!("{v:.1}")),
            (Tipo::RealReal, Valor::RealReal(prom, max)) => Ok(format!("{prom:.1}/{max:.1}")),
            (Tipo::EnteroEntero, Valor::EnteroEntero(ancho, alto)) => Ok(format!("{ancho}x{alto}")),
            _ => Err(format!(
                "el valor '{nombre}' no encaja con el tipo {self:?}"
            )),
        }
    }
}

/// Un campo declarado en el esquema.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Campo {
    /// Clave con la que aparece en la línea (`t`, `fps`, `ejec`, ...).
    pub nombre: &'static str,
    /// Cómo se imprime.
    pub tipo: Tipo,
    /// Unidad para el humano que lee el esquema. **No se imprime**.
    pub unidad: &'static str,
}

/// Valor de un campo, ya tipado, listo para imprimirse.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Valor {
    /// Cubre los tipos `entero` y `entero_s`.
    Entero(u64),
    /// Tipo `real`.
    Real(f64),
    /// Tipo `real/real`: promedio y máximo.
    RealReal(f64, f64),
    /// Tipo `entero_x_entero`: ancho y alto.
    EnteroEntero(u32, u32),
}

/// El contrato analizado: los campos en orden canónico y el sello de versión.
#[derive(Clone, Debug, PartialEq)]
pub struct Esquema {
    /// Todos los campos de todas las versiones, en orden canónico de escritura.
    campos: Vec<Campo>,
    /// Clave del sello (`formato`).
    sello_clave: &'static str,
    /// Versión que declara el sello y que este escritor emite.
    sello_version: u32,
}

impl Esquema {
    /// Campos del contrato, en el orden canónico en que se escriben.
    pub fn campos(&self) -> &[Campo] {
        &self.campos
    }

    /// Clave del sello de versión (`formato`).
    pub fn sello_clave(&self) -> &'static str {
        self.sello_clave
    }

    /// Versión del contrato que emite el escritor.
    pub fn sello_version(&self) -> u32 {
        self.sello_version
    }

    /// Busca un campo por su nombre.
    pub fn campo(&self, nombre: &str) -> Option<&Campo> {
        self.campos.iter().find(|campo| campo.nombre == nombre)
    }
}

/// Esquema ya analizado, memoizado: se analiza una sola vez por proceso.
static ESQUEMA: OnceLock<Result<Esquema, String>> = OnceLock::new();

/// Devuelve el esquema del contrato, analizado la primera vez que se pide.
pub fn esquema() -> Result<&'static Esquema, String> {
    ESQUEMA
        .get_or_init(|| analizar(ESQUEMA_TEXTO))
        .as_ref()
        .map_err(|motivo| motivo.clone())
}

/// Bloque `version` en construcción mientras se recorre el esquema.
struct Bloque {
    version: u32,
    hereda: Option<u32>,
    anade: Vec<&'static str>,
    recogidos: Vec<Campo>,
}

/// Analiza el texto del esquema.
///
/// Comprueba que cada bloque declara exactamente los campos que añade, que la
/// cadena `hereda` es consecutiva y que los nombres no se repiten. El texto es
/// `&'static str`, así que los nombres y unidades apuntan al propio literal
/// embebido y no hace falta reservar memoria para ellos.
fn analizar(texto: &'static str) -> Result<Esquema, String> {
    let mut bloques: Vec<Bloque> = Vec::new();
    let mut sello: Option<(&'static str, u32)> = None;

    for (indice, bruta) in texto.lines().enumerate() {
        let numero = indice + 1;
        let linea = bruta.trim();
        if linea.is_empty() || linea.starts_with('#') {
            continue;
        }
        let celdas: Vec<&'static str> = linea.split(" | ").map(str::trim).collect();
        match celdas.first().copied() {
            Some("version") => {
                if celdas.len() < 4 {
                    return Err(format!(
                        "línea {numero}: una fila 'version' necesita al menos 4 celdas"
                    ));
                }
                let version: u32 = celdas[1].parse().map_err(|_| {
                    format!("línea {numero}: la versión '{}' no es un entero", celdas[1])
                })?;
                let hereda_texto = celdas[2]
                    .strip_prefix("hereda:")
                    .map(str::trim)
                    .ok_or_else(|| format!("línea {numero}: falta la celda 'hereda:'"))?;
                let hereda = if hereda_texto == "-" {
                    None
                } else {
                    Some(hereda_texto.parse::<u32>().map_err(|_| {
                        format!("línea {numero}: 'hereda' no es un entero: '{hereda_texto}'")
                    })?)
                };
                let anade_texto = celdas[3]
                    .strip_prefix("anade:")
                    .map(str::trim)
                    .ok_or_else(|| format!("línea {numero}: falta la celda 'anade:'"))?;
                let anade: Vec<&'static str> = anade_texto
                    .split(',')
                    .map(str::trim)
                    .filter(|nombre| !nombre.is_empty())
                    .collect();
                if bloques.iter().any(|bloque| bloque.version == version) {
                    return Err(format!("línea {numero}: la versión {version} está repetida"));
                }
                bloques.push(Bloque {
                    version,
                    hereda,
                    anade,
                    recogidos: Vec::new(),
                });
            }
            Some("campo") => {
                if celdas.len() < 5 {
                    return Err(format!(
                        "línea {numero}: una fila 'campo' necesita 5 celdas"
                    ));
                }
                let bloque = bloques.last_mut().ok_or_else(|| {
                    format!("línea {numero}: un 'campo' antes de cualquier 'version'")
                })?;
                let tipo = Tipo::desde_texto(celdas[2]).ok_or_else(|| {
                    format!(
                        "línea {numero}: tipo desconocido '{}' en el campo '{}'",
                        celdas[2], celdas[1]
                    )
                })?;
                if bloque
                    .recogidos
                    .iter()
                    .any(|campo| campo.nombre == celdas[1])
                {
                    return Err(format!(
                        "línea {numero}: el campo '{}' está repetido en la versión {}",
                        celdas[1], bloque.version
                    ));
                }
                bloque.recogidos.push(Campo {
                    nombre: celdas[1],
                    tipo,
                    unidad: celdas[3],
                });
            }
            Some("sello") => {
                if celdas.len() < 3 {
                    return Err(format!(
                        "línea {numero}: una fila 'sello' necesita 3 celdas"
                    ));
                }
                let version: u32 = celdas[2]
                    .parse()
                    .map_err(|_| format!("línea {numero}: el sello apunta a una versión que no es un entero"))?;
                sello = Some((celdas[1], version));
            }
            Some(otra) => {
                return Err(format!("línea {numero}: fila desconocida '{otra}'"));
            }
            None => {}
        }
    }

    if bloques.is_empty() {
        return Err("el esquema no declara ninguna versión".to_string());
    }

    // Cada bloque declara lo que añade y lo que recoge: tienen que coincidir.
    for bloque in &bloques {
        let declarados: Vec<&str> = bloque.anade.clone();
        let recogidos: Vec<&str> = bloque.recogidos.iter().map(|campo| campo.nombre).collect();
        if declarados != recogidos {
            return Err(format!(
                "la versión {} declara añadir {:?} pero recoge {:?}",
                bloque.version, declarados, recogidos
            ));
        }
    }

    // La cadena `hereda` tiene que ser consecutiva: 1 sin padre, 2 de 1, 3 de 2...
    for (posicion, bloque) in bloques.iter().enumerate() {
        let esperado = if posicion == 0 {
            None
        } else {
            Some(bloques[posicion - 1].version)
        };
        if bloque.hereda != esperado {
            return Err(format!(
                "la versión {} declara heredar de {:?}, pero la anterior es {:?}",
                bloque.version, bloque.hereda, esperado
            ));
        }
    }

    let (sello_clave, sello_version) =
        sello.ok_or_else(|| "el esquema no declara el sello de versión".to_string())?;
    if !bloques.iter().any(|bloque| bloque.version == sello_version) {
        return Err(format!(
            "el sello apunta a la versión {sello_version}, que el esquema no declara"
        ));
    }

    let campos: Vec<Campo> = bloques
        .iter()
        .flat_map(|bloque| bloque.recogidos.iter().copied())
        .collect();

    Ok(Esquema {
        campos,
        sello_clave,
        sello_version,
    })
}

/// Escribe una línea de segundo completa a partir de los valores de sus campos.
///
/// Recorre el esquema en orden canónico, imprime cada valor con la forma de su
/// tipo y remata la línea con el sello (`formato=2`). Falla nombrando el campo en
/// los tres casos que puede haber: falta un valor, sobra un valor que el esquema
/// no declara, o el valor no encaja con el tipo declarado.
pub fn escribir_linea(valores: &BTreeMap<&'static str, Valor>) -> Result<String, String> {
    let esquema = esquema()?;

    let mut piezas: Vec<String> = Vec::with_capacity(esquema.campos().len() + 1);
    for campo in esquema.campos() {
        let valor = valores
            .get(campo.nombre)
            .ok_or_else(|| format!("falta el valor del campo '{}'", campo.nombre))?;
        let texto = campo.tipo.formatear(campo.nombre, valor)?;
        piezas.push(format!("{}={}", campo.nombre, texto));
    }

    for nombre in valores.keys() {
        if esquema.campo(nombre).is_none() {
            return Err(format!("el valor '{nombre}' no está en el esquema"));
        }
    }

    piezas.push(format!(
        "{}={}",
        esquema.sello_clave(),
        esquema.sello_version()
    ));
    Ok(piezas.join(" "))
}

#[cfg(test)]
#[path = "telemetria_tests.rs"]
mod tests;
