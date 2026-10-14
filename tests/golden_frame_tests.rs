//! Pruebas de integración del arnés de fotogramas dorados
//! (propuesta `docs/propuestas/2026-10-13_arnes-de-fotogramas-dorados.md`).
//!
//! Comprueban `scripts/diagnostico/fotogramas_dorados.py` como lo usaría una persona:
//! invocándolo desde la raíz del repositorio y leyendo su código de salida y su informe.
//! No necesitan ROM ni BIOS: los cuatro tests trabajan con PPM sintéticos y con un
//! manifiesto de dorados falso escrito en un directorio temporal.
//!
//! El arnés es una herramienta de Python, así que `python` debe estar en el PATH. A
//! diferencia de `packaging_tests.rs`, que omite la comprobación si falta Python, aquí
//! no se omite nada: sin Python el arnés no puede comprobarse, y el test tiene que
//! fallar en voz alta.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

/// Resuelve la raíz del repositorio de forma relativa al manifiesto de Cargo.
fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

/// Ruta del arnés, tal como lo invoca una persona.
fn arnes() -> PathBuf {
    repo_root()
        .join("scripts")
        .join("diagnostico")
        .join("fotogramas_dorados.py")
}

/// Directorio temporal propio de cada test, limpio al empezar.
fn dir_temporal(nombre: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!(
        "br-doradas-{}-{}",
        nombre,
        std::process::id()
    ));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).expect("el directorio temporal debe crearse");
    dir
}

/// Rellena un framebuffer RGB de 240x160 con un color uniforme.
fn relleno_uniforme(color: [u8; 3]) -> Vec<u8> {
    let mut rgb = Vec::with_capacity(240 * 160 * 3);
    for _ in 0..240 * 160 {
        rgb.extend_from_slice(&color);
    }
    rgb
}

/// Escribe los píxeles como PPM P6, el formato exacto que escribe el motor (`dump_frame`).
fn escribir_ppm(dir: &Path, nombre: &str, rgb: &[u8]) -> PathBuf {
    assert_eq!(rgb.len(), 240 * 160 * 3, "el framebuffer de prueba debe tener 38400 pixeles");
    let mut bytes = b"P6\n240 160\n255\n".to_vec();
    bytes.extend_from_slice(rgb);
    let ruta = dir.join(nombre);
    fs::write(&ruta, bytes).expect("el PPM de prueba debe escribirse");
    ruta
}

/// Ejecuta el arnés desde la raíz del repositorio, como lo haría una persona.
fn correr(args: &[&str]) -> Output {
    Command::new("python")
        .arg(arnes())
        .args(args)
        .current_dir(repo_root())
        .output()
        .expect("python debe estar en el PATH: el arnés de fotogramas dorados es una herramienta de Python")
}

/// Salida completa (salida estándar y de error) para poder afirmar sobre el mensaje.
fn salida(completada: &Output) -> String {
    format!(
        "{}{}",
        String::from_utf8_lossy(&completada.stdout),
        String::from_utf8_lossy(&completada.stderr)
    )
}

fn codigo_de_salida(completada: &Output) -> i32 {
    completada
        .status
        .code()
        .expect("el proceso del arnés terminó con código de salida")
}

/// Decodificador independiente del mapa PNG, escrito por el propio test en su directorio
/// temporal y ejecutado con Python. Es una segunda implementación (lectura) frente a la
/// del arnés (escritura), así que un PNG mal formado no puede pasar desapercibido.
fn decodificador_png() -> &'static str {
    r#"
import sys, zlib, struct
d = open(sys.argv[1], "rb").read()
assert d[:8] == b"\x89PNG\r\n\x1a\n", "no es un PNG"
pos, idat, w, h = 8, b"", 0, 0
while pos < len(d):
    lon = struct.unpack(">I", d[pos:pos+4])[0]
    tipo = d[pos+4:pos+8]
    cuerpo = d[pos+8:pos+8+lon]
    pos += 12 + lon
    if tipo == b"IHDR":
        w, h, profundidad, tipo_color = struct.unpack(">IIBB", cuerpo[:10])
        assert (profundidad, tipo_color) == (8, 2), "el mapa debe ser RGB8 sin alfa"
    elif tipo == b"IDAT":
        idat += cuerpo
    elif tipo == b"IEND":
        break
crudo = zlib.decompress(idat)
ancho_b = w * 3
filas = []
for y in range(h):
    filtro = crudo[y * (ancho_b + 1)]
    linea = bytearray(crudo[y * (ancho_b + 1) + 1:(y + 1) * (ancho_b + 1)])
    for x in range(ancho_b):
        a = linea[x - 3] if x >= 3 else 0
        b = filas[y - 1][x] if y >= 1 else 0
        c = filas[y - 1][x - 3] if (y >= 1 and x >= 3) else 0
        if filtro == 1:
            linea[x] = (linea[x] + a) & 0xFF
        elif filtro == 2:
            linea[x] = (linea[x] + b) & 0xFF
        elif filtro == 3:
            linea[x] = (linea[x] + (a + b) // 2) & 0xFF
        elif filtro == 4:
            p = a + b - c
            pa, pb, pc = abs(p - a), abs(p - b), abs(p - c)
            pr = a if (pa <= pb and pa <= pc) else (b if pb <= pc else c)
            linea[x] = (linea[x] + pr) & 0xFF
        elif filtro != 0:
            raise SystemExit("filtro de fila desconocido %d" % filtro)
    filas.append(linea)
rojos = [(x // 3, y) for y in range(h) for x in range(0, ancho_b, 3)
         if tuple(filas[y][x:x + 3]) == (255, 0, 0)]
grises = sum(1 for y in range(h) for x in range(0, ancho_b, 3)
             if filas[y][x] == filas[y][x + 1] == filas[y][x + 2])
print("%dx%d rojos=%d primeros=%s grises=%d" % (w, h, len(rojos), rojos[:3], grises))
"#
}

/// Pregunta al decodificador auxiliar y devuelve su salida.
fn leer_mapa(dir: &Path, mapa: &Path) -> String {
    let script = dir.join("leer_png.py");
    fs::write(&script, decodificador_png()).expect("el decodificador auxiliar debe escribirse");
    let out = Command::new("python")
        .arg(&script)
        .arg(mapa)
        .current_dir(repo_root())
        .output()
        .expect("python debe estar en el PATH");
    assert!(
        out.status.success(),
        "el mapa PNG debe decodificarse: {}",
        salida(&out)
    );
    salida(&out)
}

// ---------------------------------------------------------------- escenario normal

/// Un cambio de un solo píxel tiene que salir localizado: cuántos, en qué columna y fila,
/// qué caja ocupan y un mapa PNG con ese píxel en rojo puro sobre un fondo gris.
#[test]
fn el_diferenciador_indica_el_pixel_que_cambia() {
    let dir = dir_temporal("localiza");
    let mut rgb_b = relleno_uniforme([10, 20, 30]);
    let indice = (18 * 240 + 64) * 3;
    rgb_b[indice..indice + 3].copy_from_slice(&[200, 201, 202]);
    let a = escribir_ppm(&dir, "a.ppm", &relleno_uniforme([10, 20, 30]));
    let b = escribir_ppm(&dir, "b.ppm", &rgb_b);
    let mapa = dir.join("m.png");

    let out = correr(&[
        "diferenciar",
        a.to_str().unwrap(),
        b.to_str().unwrap(),
        "--mapa",
        mapa.to_str().unwrap(),
    ]);
    let texto = salida(&out);

    assert_eq!(
        codigo_de_salida(&out),
        1,
        "un pixel cambiado es una regresion (codigo 1), no un error de entrada. Informe:\n{texto}"
    );
    assert!(texto.contains("DIFIERE 1 px"), "el informe debe contar los pixeles: {texto}");
    assert!(texto.contains("columna=64"), "el informe debe nombrar la columna: {texto}");
    assert!(texto.contains("fila=18"), "el informe debe nombrar la fila: {texto}");
    assert!(
        texto.contains("caja x64-64 y18-18"),
        "el informe debe dar la caja envolvente: {texto}"
    );

    assert!(mapa.exists(), "el mapa de diferencias debe escribirse");
    let decodificado = leer_mapa(&dir, &mapa);
    assert!(
        decodificado.contains("240x160 rojos=1 primeros=[(64, 18)]"),
        "el mapa debe tener 240x160 pixeles con un solo rojo en (64, 18). Obtenido: {decodificado}"
    );
}

// ---------------------------------------------------------------- escenario limite

/// Dos fotogramas idénticos no deben producir ni veredicto de regresión ni mapa.
#[test]
fn dos_fotogramas_identicos_no_reportan_diferencias() {
    let dir = dir_temporal("identicos");
    let a = escribir_ppm(&dir, "a.ppm", &relleno_uniforme([10, 20, 30]));
    let b = escribir_ppm(&dir, "b.ppm", &relleno_uniforme([10, 20, 30]));
    let mapa = dir.join("no_debe_existir.png");

    let out = correr(&[
        "diferenciar",
        a.to_str().unwrap(),
        b.to_str().unwrap(),
        "--mapa",
        mapa.to_str().unwrap(),
    ]);
    let texto = salida(&out);

    assert_eq!(codigo_de_salida(&out), 0, "dos fotogramas iguales no son una regresion: {texto}");
    assert!(texto.contains("sin diferencias"), "el informe debe decirlo: {texto}");
    assert!(!mapa.exists(), "sin diferencias no debe escribirse mapa");
}

// ---------------------------------------------------------------- escenario de error

/// Un archivo que no es un PPM P6 de 240x160 tiene que producir un error de entrada
/// (código 2), nunca un veredicto de diferencias.
#[test]
fn un_ppm_corrupto_no_se_confunde_con_una_regresion() {
    let dir = dir_temporal("corrupto");
    let buena = escribir_ppm(&dir, "buena.ppm", &relleno_uniforme([10, 20, 30]));

    // Cabecera equivocada: P5 (mapa de grises) en vez de P6 (RGB).
    let p5 = dir.join("p5.ppm");
    let mut bytes_p5 = b"P5\n240 160\n255\n".to_vec();
    bytes_p5.extend(std::iter::repeat_n(0u8, 240 * 160));
    fs::write(&p5, bytes_p5).expect("el PPM P5 de prueba debe escribirse");

    let out = correr(&["diferenciar", buena.to_str().unwrap(), p5.to_str().unwrap()]);
    let texto = salida(&out);
    assert_eq!(codigo_de_salida(&out), 2, "una cabecera distinta de P6 es error de entrada: {texto}");
    assert!(texto.contains("cabecera"), "el mensaje debe nombrar el problema: {texto}");

    // Payload incompleto: 38399 pixeles en vez de 38400.
    let corto = dir.join("corto.ppm");
    let mut bytes_cortos = b"P6\n240 160\n255\n".to_vec();
    bytes_cortos.extend(std::iter::repeat_n(0u8, 38399 * 3));
    fs::write(&corto, bytes_cortos).expect("el PPM corto de prueba debe escribirse");

    let out = correr(&["diferenciar", buena.to_str().unwrap(), corto.to_str().unwrap()]);
    let texto = salida(&out);
    assert_eq!(codigo_de_salida(&out), 2, "un PPM incompleto es error de entrada: {texto}");
    assert!(texto.contains("38400"), "el mensaje debe nombrar el tamaño esperado: {texto}");
}

// ---------------------------------------------------------------- escenario de regresion

/// Dorados capturados con otro guion de entrada no sirven para comparar: tienen que
/// rechazarse nombrando el campo, antes de intentar ejecutar el binario candidato.
#[test]
fn los_dorados_desactualizados_no_se_comparan_en_vano() {
    let dir = dir_temporal("desactualizados");
    let dorados = dir.join("dorados");
    fs::create_dir_all(&dorados).expect("la carpeta de dorados de prueba debe crearse");

    // Guion de entrada falso (formato gba-input v1) y ficheros de ROM y BIOS fingidos:
    // para el rechazo solo importan sus hashes.
    let guion = dir.join("escena.input");
    fs::write(&guion, "gba-input v1\n120 3f7\n128 3ff\n").expect("el guion de prueba debe escribirse");
    let rom = dir.join("rom.gba");
    let bios = dir.join("bios.bin");
    fs::write(&rom, vec![0u8; 64]).expect("la ROM de prueba debe escribirse");
    fs::write(&bios, vec![1u8; 64]).expect("la BIOS de prueba debe escribirse");

    // El manifiesto guardado coincide con la ROM y la BIOS actuales, pero no con el guion.
    let sha = |ruta: &Path| {
        let bytes = fs::read(ruta).expect("los ficheros de prueba se pueden leer");
        let mut h = 0xcbf2_9ce4_8422_2325u64;
        for b in bytes {
            h ^= b as u64;
            h = h.wrapping_mul(0x1_0000_01b3);
        }
        format!("{h:016x}")
    };
    let manifest = format!(
        "{{\n  \"version\": 1,\n  \"rom_sha256\": \"{}\",\n  \"bios_sha256\": \"{}\",\n  \
         \"script_sha256\": \"0000000000000000\",\n  \"escenas\": {{}}\n}}\n",
        sha(&rom),
        sha(&bios)
    );
    fs::write(dorados.join("manifesto.json"), manifest).expect("el manifiesto debe escribirse");

    let toml = dir.join("escenas.toml");
    fs::write(&toml, "version = 1\n\n[[escena]]\nnombre = \"falsa\"\nscript = \"escena.input\"\nanclas = [120]\n")
        .expect("el manifiesto de escenas debe escribirse");

    // El binario no existe: si el arnés llegara a ejecutarlo, fallaría por otro motivo y el
    // test lo delataría.
    let out = correr(&[
        "comparar",
        "--bin",
        dir.join("no_existe.exe").to_str().unwrap(),
        "--rom",
        rom.to_str().unwrap(),
        "--bios",
        bios.to_str().unwrap(),
        "--escenas",
        toml.to_str().unwrap(),
        "--dorados",
        dorados.to_str().unwrap(),
    ]);
    let texto = salida(&out);

    assert_eq!(
        codigo_de_salida(&out),
        2,
        "dorados que no corresponden son error de entrada (codigo 2), no una regresion: {texto}"
    );
    assert!(
        texto.contains("los dorados no corresponden") && texto.contains("script"),
        "el mensaje debe rechazarlos nombrando el campo: {texto}"
    );
}
