//! Pruebas del arnés compartido de los experimentos de `examples/`
//! (propuesta `docs/propuestas/2026-10-22_arnes-de-experimentos-comun.md`).
//!
//! Los 18 experimentos comparten `examples/comun/`, que vive **fuera** del crate que se
//! empaqueta en el APK. Eso lo deja sin `cargo test --lib`, así que se incluye aquí con
//! `#[path]`: es el mismo fichero que compilan los ejemplos, probado de verdad.
//!
//! Se incluye `entrada.rs` suelto y no `mod.rs` entero: `gpu.rs` arrastra wgpu y los
//! pipelines del motor, que este test no necesita para probar el parser.

//! Los escenarios de estructura (inventario y ausencia de copias) leen `examples/` y
//! `Cargo.toml` con `std::fs`; no compilan ni ejecutan ningún experimento, así que no
//! necesitan ROM, BIOS ni GPU.

use std::collections::BTreeSet;
use std::fs;
use std::path::PathBuf;

// `leer_tramos` sólo lo usa `ppu_compare`; aquí se incluye el fichero entero.
#[allow(dead_code)]
#[path = "../examples/comun/entrada.rs"]
mod entrada;

// `diagnostico.rs` usa `super::color` y `super::registro`: se incluyen los dos sueltos en
// la raíz del test para que esas rutas resuelvan igual que dentro de `comun/`. Ninguno
// arrastra wgpu, así que el test no necesita GPU ni la feature `proto`.
#[allow(dead_code)]
#[path = "../examples/comun/color.rs"]
mod color;
#[allow(dead_code)]
#[path = "../examples/comun/registro.rs"]
mod registro;
#[allow(dead_code)]
#[path = "../examples/comun/diagnostico.rs"]
mod diagnostico;

/// Raíz del repositorio, relativa al manifiesto de Cargo.
fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn dir_examples() -> PathBuf {
    repo_root().join("examples")
}

/// Los 18 ficheros de experimento, ordenados por nombre.
fn experimentos() -> Vec<PathBuf> {
    let mut rutas: Vec<PathBuf> = fs::read_dir(dir_examples())
        .expect("examples/ tiene que existir")
        .filter_map(|entrada| entrada.ok())
        .map(|entrada| entrada.path())
        .filter(|ruta| ruta.is_file() && ruta.extension().is_some_and(|ext| ext == "rs"))
        .collect();
    rutas.sort();
    rutas
}

fn leer(ruta: &PathBuf) -> String {
    fs::read_to_string(ruta).unwrap_or_else(|e| panic!("no se pudo leer {}: {e}", ruta.display()))
}

#[test]
fn test_los_experimentos_usan_el_modulo_comun() {
    let rutas = experimentos();
    assert_eq!(rutas.len(), 18, "tienen que quedar los 18 experimentos");

    // Lo que deja de poder estar declarado dentro de un experimento: si reaparece una
    // copia, el arreglo del arnés vuelve a tener que hacerse 47 veces.
    let declaraciones_prohibidas = [
        "fn leer_secuencia",
        "const BOTONES",
        "fn canal_8bit",
        "fn cerca(",
        "struct Gpu",
        "fn io16",
        "fn io_con_variante",
        "fn software_a_rgb",
    ];

    for ruta in &rutas {
        let nombre = ruta.file_name().unwrap().to_string_lossy().into_owned();
        let texto = leer(ruta);

        assert!(
            texto.contains("mod comun;"),
            "{nombre}: tiene que declarar el módulo común"
        );

        for prohibida in declaraciones_prohibidas {
            assert!(
                !texto.contains(prohibida),
                "{nombre}: vuelve a declarar '{prohibida}', que ya vive en examples/comun/"
            );
        }
    }

    // `dispcnt_de` tiene una única definición legítima fuera del módulo: la de
    // `ppu_capas_linea.rs`, que resuelve 11 variantes desde una tabla y no es una copia.
    let con_dispcnt: Vec<String> = rutas
        .iter()
        .filter(|ruta| leer(ruta).contains("fn dispcnt_de"))
        .map(|ruta| ruta.file_name().unwrap().to_string_lossy().into_owned())
        .collect();
    assert_eq!(
        con_dispcnt,
        vec!["ppu_capas_linea.rs".to_string()],
        "sólo ppu_capas_linea conserva su propia variante de dispcnt_de"
    );

    // Y el módulo tiene que declarar de verdad lo que los experimentos dejaron de declarar.
    let esperado = [
        ("mod.rs", "pub mod entrada;"),
        ("mod.rs", "pub mod gpu;"),
        ("entrada.rs", "pub fn leer_secuencia"),
        ("entrada.rs", "pub fn leer_tramos"),
        ("color.rs", "pub fn canal_8bit"),
        ("color.rs", "pub fn cerca("),
        ("color.rs", "pub fn software_a_rgb"),
        ("registro.rs", "pub fn io16"),
        ("registro.rs", "pub fn dispcnt_de"),
        ("registro.rs", "pub fn io_con_variante"),
        ("gpu.rs", "pub struct Gpu"),
        ("gpu.rs", "pub fn renderiza"),
        ("mod.rs", "pub mod diagnostico;"),
        ("diagnostico.rs", "pub fn guardar_captura_si_difiere"),
        ("diagnostico.rs", "pub fn diagnosticar"),
        ("diagnostico.rs", "pub fn lienzo_doble_bgrx"),
        ("diagnostico.rs", "pub fn guardar_captura_forzada"),
        ("diagnostico.rs", "pub fn render_software_fin"),
        ("diagnostico.rs", "pub fn evaluar_cuadro_en_vivo"),
    ];
    for (fichero, fragmento) in esperado {
        let ruta = dir_examples().join("comun").join(fichero);
        assert!(ruta.is_file(), "falta examples/comun/{fichero}");
        assert!(
            leer(&ruta).contains(fragmento),
            "examples/comun/{fichero} no declara '{fragmento}'"
        );
    }
}

#[test]
fn test_el_inventario_coincide_con_cargo_toml() {
    let en_disco: BTreeSet<String> = experimentos()
        .iter()
        .map(|ruta| ruta.file_stem().unwrap().to_string_lossy().into_owned())
        .collect();

    // Dos de los 18 compilan sin la feature: `ppu_barrido_nativa` y `ppu_modo_cuadro`
    // no usan la captura por línea de `perf-video` (medido: `cargo build --example
    // ppu_barrido_nativa --example ppu_modo_cuadro` termina con Finished). Los demás sí.
    const SIN_FEATURE: [&str; 2] = ["ppu_barrido_nativa", "ppu_modo_cuadro"];

    let cargo = leer(&repo_root().join("Cargo.toml"));
    let mut declarados = BTreeSet::new();
    for bloque in cargo.split("[[example]]").skip(1) {
        // El bloque acaba en el siguiente encabezado de sección.
        let fin = bloque.find("\n[").unwrap_or(bloque.len());
        let cuerpo = &bloque[..fin];
        let nombre = cuerpo
            .lines()
            .find_map(|linea| linea.trim().strip_prefix("name = "))
            .map(|valor| valor.trim().trim_matches('"').to_string())
            .unwrap_or_else(|| panic!("un bloque [[example]] no declara name: {cuerpo}"));
        let exige = cuerpo.contains("required-features = [\"proto\"]");
        let exento = SIN_FEATURE.contains(&nombre.as_str());
        assert!(
            exige != exento,
            "{nombre}: o sobra required-features = [\"proto\"] o falta; los dos casos son incompatibles"
        );
        declarados.insert(nombre);
    }

    assert_eq!(declarados.len(), 18, "Cargo.toml declara los 18 experimentos");
    assert_eq!(
        declarados, en_disco,
        "los nombres de Cargo.toml y los ficheros de examples/ tienen que coincidir: \
         las propuestas antiguas los citan por nombre"
    );

    let con_feature = declarados
        .iter()
        .filter(|nombre| !SIN_FEATURE.contains(&nombre.as_str()))
        .count();
    assert_eq!(con_feature, 16, "16 experimentos exigen la feature proto");
}

#[test]
fn test_la_secuencia_se_lee_igual_que_antes() {
    // El mismo texto que tragaba cualquiera de las 45 copias: comentario, tramo con
    // `ninguno`, línea en blanco y dos tramos con varios botones.
    let tramos = entrada::leer_secuencia("# comentario\n120 ninguno\n\n60 A+B\n30 R\n")
        .expect("la secuencia es válida");

    // A = bit 0, B = bit 1, R = bit 8 (orden de KEYINPUT, activo en bajo al aplicarlo).
    assert_eq!(tramos, vec![(120, 0x000), (60, 0x003), (30, 0x100)]);

    // Y un caso por botón: con sólo `A+B` el intercambio de A y B pasaría inadvertido,
    // porque los dos juntos dan el mismo par de bits.
    let por_boton = [
        ("A", 0x001u16),
        ("B", 0x002),
        ("Select", 0x004),
        ("Start", 0x008),
        ("Right", 0x010),
        ("Left", 0x020),
        ("Up", 0x040),
        ("Down", 0x080),
        ("R", 0x100),
        ("L", 0x200),
    ];
    for (boton, bits) in por_boton {
        let tramos = entrada::leer_secuencia(&format!("1 {boton}\n"))
            .unwrap_or_else(|e| panic!("'{boton}' tiene que ser un botón válido: {e}"));
        assert_eq!(tramos, vec![(1, bits)], "'{boton}' ocupa otro bit de KEYINPUT");
    }
}

#[test]
fn test_la_secuencia_rechaza_un_boton_desconocido() {
    let error = entrada::leer_secuencia("10 A+Foo\n")
        .expect_err("un botón que no existe tiene que ser un error");
    assert!(
        error.contains("línea 1") && error.contains("Foo"),
        "el mensaje tiene que nombrar la línea y el botón; fue: {error}"
    );
}

// ===========================================================================
// Captura visual bajo demanda y diagnóstico causal
// (propuesta `docs/propuestas/2026-10-22_arnes-comparacion-visual-y-causal.md`)
// ===========================================================================

const PX: usize = 240 * 160;

/// Imagen RGB de 240x160 rellena de un gris medio.
fn imagen_gris() -> Vec<u8> {
    vec![0x80; PX * 3]
}

fn pinta(img: &mut [u8], x: usize, y: usize, rgb: [u8; 3]) {
    let o = (y * 240 + x) * 3;
    img[o..o + 3].copy_from_slice(&rgb);
}

/// Carpeta temporal propia de cada test, vaciada al empezar.
fn carpeta_temporal(nombre: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("blazing-arnes-{nombre}-{}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).expect("la carpeta temporal tiene que poder crearse");
    dir
}

fn archivos_en(dir: &PathBuf) -> usize {
    fs::read_dir(dir).map(|it| it.count()).unwrap_or(0)
}

#[test]
fn test_captura_compuesta_solo_se_escribe_cuando_hay_diferencias() {
    let dir = carpeta_temporal("captura");

    // Par 1: todos los canales difieren a lo sumo en TOLERANCIA (1): no hay diferencia.
    let sw = imagen_gris();
    let mut casi = imagen_gris();
    for p in (0..PX).step_by(7) {
        casi[p * 3] = 0x81;
        casi[p * 3 + 2] = 0x7F;
    }
    let r = diagnostico::guardar_captura_si_difiere(&dir, 7500, "fin", &sw, &casi)
        .expect("sin diferencias no hay error de escritura");
    assert!(r.is_none(), "sin diferencias no debe devolver resumen");
    assert_eq!(archivos_en(&dir), 0, "sin diferencias no se escribe ningún archivo");

    // Par 2: (10, 20) difiere 20 niveles y (50, 60) difiere 100.
    let mut nat = imagen_gris();
    pinta(&mut nat, 10, 20, [0x80 + 20, 0x80, 0x80]);
    pinta(&mut nat, 50, 60, [0x80, 0x80 - 100, 0x80]);
    let resumen = diagnostico::guardar_captura_si_difiere(&dir, 5300, "linea", &sw, &nat)
        .expect("la escritura tiene que funcionar")
        .expect("con diferencias tiene que devolver un resumen");

    assert_eq!(resumen.distintos, 2);
    assert_eq!(resumen.caja, (10, 20, 50, 60), "caja (x0, y0, x1, y1)");
    assert_eq!(resumen.primero, (10, 20));
    assert_eq!(resumen.max_error, (50, 60));
    assert_eq!(resumen.ruta, dir.join("cuadro-5300-linea.png"));
    assert_eq!(archivos_en(&dir), 1, "una única imagen compuesta por caso");

    let png = fs::read(&resumen.ruta).expect("el PNG tiene que existir");
    assert_eq!(&png[..8], b"\x89PNG\r\n\x1a\n", "firma PNG");
    assert_eq!(&png[12..16], b"IHDR");
    let ancho = u32::from_be_bytes([png[16], png[17], png[18], png[19]]);
    let alto = u32::from_be_bytes([png[20], png[21], png[22], png[23]]);
    assert_eq!((ancho, alto), (960, 160), "cuatro paneles de 240x160 en fila");

    // El segundo caso del mismo cuadro va a la misma carpeta, junto al primero.
    diagnostico::guardar_captura_si_difiere(&dir, 5300, "fin", &sw, &nat)
        .expect("la escritura tiene que funcionar")
        .expect("con diferencias tiene que devolver un resumen");
    assert_eq!(archivos_en(&dir), 2, "los dos casos conviven en la misma carpeta");

    let _ = fs::remove_dir_all(&dir);
}

/// I/O de 1 KiB con `DISPCNT` y `BG0HOFS` dados.
fn io_con(dispcnt: u16, bg0hofs: u16) -> Vec<u8> {
    let mut io = vec![0u8; 0x400];
    io[0..2].copy_from_slice(&dispcnt.to_le_bytes());
    io[0x10..0x12].copy_from_slice(&bg0hofs.to_le_bytes());
    io
}

#[test]
fn test_diagnostico_causal_identifica_origen_de_la_diferencia() {
    let paleta = vec![0u8; 0x400];
    let oam_vacia = vec![0u8; 0x400];

    // Caso 1: la nativa coincide con el estado de fin de cuadro; la línea 40 tenía otro
    // BG0HOFS y otra VRAM. La causa es el estado que cambia a mitad de cuadro.
    let io_fin = io_con(0x1F00, 0x0000);
    let vram_fin = vec![0u8; 0x18000];
    let io_l = io_con(0x1F00, 0x0008);
    let mut vram_l = vram_fin.clone();
    vram_l[0x100] = 0xAB;

    let sw_fin = imagen_gris();
    let nativa = sw_fin.clone();
    let mut sw_linea = imagen_gris();
    for x in 0..8 {
        pinta(&mut sw_linea, x, 40, [0, 0, 0]);
    }

    let fin = diagnostico::EstadoRef { io: &io_fin, palette: &paleta, vram: &vram_fin, oam: &oam_vacia };
    let mut lineas = vec![fin; 160];
    lineas[40] = diagnostico::EstadoRef { io: &io_l, palette: &paleta, vram: &vram_l, oam: &oam_vacia };

    let mut llamadas = 0usize;
    let d = diagnostico::diagnosticar(&nativa, &sw_fin, &sw_linea, fin, &lineas, |_, _| {
        llamadas += 1;
        0
    });
    assert_eq!(d.causa, "estado_por_linea");
    assert!(d.componentes.contains(&"vram"), "componentes: {:?}", d.componentes);
    assert!(!d.componentes.contains(&"oam"), "la OAM no cambió: {:?}", d.componentes);
    assert!(d.registros.contains(&"BG0HOFS"), "registros: {:?}", d.registros);
    assert_eq!(llamadas, 0, "sin fallo de dibujo no se corre la ablación");

    // Caso 2: la nativa difiere del estado de fin de cuadro en 40 píxeles de la línea 34
    // (x 12..52). OAM[3] es un sprite 8x8 en (12, 34). Sin OBJWIN el residuo cae a 0.
    let mut oam = vec![0u8; 0x400];
    oam[3 * 8..3 * 8 + 2].copy_from_slice(&34u16.to_le_bytes()); // y = 34, normal, 8x8
    oam[3 * 8 + 2..3 * 8 + 4].copy_from_slice(&12u16.to_le_bytes()); // x = 12
    let io_win = io_con(0x9F00, 0); // OBJWIN + todas las capas
    let fin2 = diagnostico::EstadoRef { io: &io_win, palette: &paleta, vram: &vram_fin, oam: &oam };
    let lineas2 = vec![fin2; 160];
    let mut nativa2 = imagen_gris();
    for x in 12..52 {
        pinta(&mut nativa2, x, 34, [0xFF, 0, 0]);
    }

    let mut vistas: Vec<String> = Vec::new();
    let d2 = diagnostico::diagnosticar(&nativa2, &sw_fin, &sw_fin, fin2, &lineas2, |nombre, io_v| {
        vistas.push(nombre.to_string());
        // La variante «sin OBJWIN» tiene que llegar con el bit 15 de DISPCNT apagado.
        if io_v[1] & 0x80 == 0 { 0 } else { 40 }
    });
    assert_eq!(d2.causa, "render_fin_de_cuadro");
    assert_eq!(d2.dif_fin, 40);
    assert_eq!(d2.resuelve, vec!["sin OBJWIN".to_string()], "residuos: {:?}", d2.residuos);
    assert!(vistas.contains(&"sin BLDCNT".to_string()), "la ablación prueba la mezcla");
    let testigo = d2.testigo.as_ref().expect("tiene que haber píxel testigo");
    assert_eq!((testigo.x, testigo.y), (12, 34));
    assert_eq!(testigo.sprites, vec![3], "sólo OAM[3] cubre (12, 34)");
}

#[test]
fn test_deduplicacion_de_barrido_omite_firmas_repetidas() {
    let firma = |caja| diagnostico::Firma {
        caso: "fin".to_string(),
        caja,
        dispcnt: 0x9F40,
        bldcnt: 0x3F40,
    };
    let mut filtro = diagnostico::FiltroFirmas::default();
    let decisiones: Vec<bool> = [
        firma((10, 20, 50, 60)),
        firma((10, 20, 50, 60)),
        firma((10, 20, 50, 60)),
        firma((0, 0, 239, 159)),
    ]
    .into_iter()
    .map(|f| filtro.capturar(f))
    .collect();
    assert_eq!(decisiones, vec![true, false, false, true]);
    assert_eq!(filtro.omitidas, 2, "las repeticiones se cuentan");

    // Otro caso tiene su propia historia: no lo tapa la firma de «fin».
    let mut otra = firma((0, 0, 239, 159));
    otra.caso = "linea".to_string();
    assert!(filtro.capturar(otra));
}


#[test]
fn test_lienzo_doble_coloca_software_izquierda_y_nativa_derecha() {
    let mut sw = vec![0u8; 240 * 160 * 3];
    let mut nat = vec![0u8; 240 * 160 * 3];

    // (5, 10): sw rojo (255, 0, 0), nat azul (0, 0, 255)
    let idx_rojo = (10 * 240 + 5) * 3;
    sw[idx_rojo] = 255;
    nat[idx_rojo + 2] = 255;

    // (20, 30): idéntico verde (0, 255, 0)
    let idx_verde = (30 * 240 + 20) * 3;
    sw[idx_verde + 1] = 255;
    nat[idx_verde + 1] = 255;

    let doble_normal = diagnostico::lienzo_doble_bgrx(&sw, &nat, false);
    assert_eq!(doble_normal.len(), 480 * 160);

    // En Windows BGRX: rojo es 0x00FF0000, azul es 0x000000FF, verde es 0x0000FF00
    // Mitad izquierda (software): (5, 10)
    assert_eq!(doble_normal[10 * 480 + 5], 0x00FF0000, "sw a la izquierda (5, 10)");
    // Mitad derecha (nativa): (240 + 5, 10)
    assert_eq!(doble_normal[10 * 480 + 240 + 5], 0x000000FF, "nat a la derecha (245, 10)");

    // Con resaltado activo: el píxel distinto en la mitad derecha se pinta de magenta 0x00FF00FF
    let doble_resaltado = diagnostico::lienzo_doble_bgrx(&sw, &nat, true);
    assert_eq!(doble_resaltado[10 * 480 + 240 + 5], 0x00FF00FF, "distinto resaltado magenta");
    assert_eq!(doble_resaltado[30 * 480 + 240 + 20], 0x0000FF00, "idéntico conserva verde");
}

#[test]
fn test_visor_filtra_firmas_en_vivo_y_permite_captura_forzada() {
    let dir = std::env::temp_dir().join(format!("test_visor_vivo_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();

    let sw = vec![128u8; 240 * 160 * 3];
    let nat_igual = sw.clone();
    let mut nat_dif = sw.clone();
    nat_dif[0] = 200; // 1 pixel distinto

    let mut filtro = diagnostico::FiltroFirmas::default();

    // Cuadro 1: 0 diferencias, no forzado -> no captura
    let dec1 = diagnostico::decidir_captura_en_vivo(1, &sw, &nat_igual, 0x9F40, 0x3F40, false, &mut filtro);
    assert!(dec1.is_none(), "cuadro 1 limpio no captura");

    // Cuadro 2: dif, no forzado -> captura
    let dec2 = diagnostico::decidir_captura_en_vivo(2, &sw, &nat_dif, 0x9F40, 0x3F40, false, &mut filtro);
    assert!(dec2.is_some(), "cuadro 2 dif captura");
    let guardado2 = diagnostico::guardar_captura_si_difiere(&dir, 2, "fin", &sw, &nat_dif).unwrap();
    assert!(guardado2.is_some());
    assert!(dir.join("cuadro-2-fin.png").is_file());

    // Cuadro 3: misma dif, no forzado -> omitido por firma repetida
    let dec3 = diagnostico::decidir_captura_en_vivo(3, &sw, &nat_dif, 0x9F40, 0x3F40, false, &mut filtro);
    assert!(dec3.is_none(), "cuadro 3 repetido se omite");

    // Cuadro 4: 0 diferencias, forzado manual (F5) -> captura forzada
    let dec4 = diagnostico::decidir_captura_en_vivo(4, &sw, &nat_igual, 0x9F40, 0x3F40, true, &mut filtro);
    assert_eq!(dec4, Some("manual".to_string()));
    let ruta_manual = diagnostico::guardar_captura_forzada(&dir, 4, "manual", &sw, &nat_igual).unwrap();
    assert!(ruta_manual.is_file());
    assert_eq!(ruta_manual.file_name().unwrap(), "cuadro-4-manual.png");

    // Limpieza
    let _ = std::fs::remove_dir_all(&dir);
}


#[test]
fn test_visor_separa_sw_fin_de_vibracion_por_scroll_en_linea() {
    let mut io_fin = vec![0u8; 0x60];
    // DISPCNT modo 0 con BG0 activo
    io_fin[0x00] = 0x00;
    io_fin[0x01] = 0x01;
    // BG0VOFS = 1 al final del cuadro (vibró 1 píxel hacia abajo)
    io_fin[0x12] = 1;

    let mut io_linea0 = io_fin.clone();
    io_linea0[0x12] = 0; // en la línea 0 valía 0

    let paleta = vec![0u8; 1024];
    let vram = vec![0u8; 96 * 1024];
    let oam = vec![0u8; 1024];

    let fin = diagnostico::EstadoRef {
        io: &io_fin,
        palette: &paleta,
        vram: &vram,
        oam: &oam,
    };
    let mut lineas = vec![fin; 160];
    let linea0_ref = diagnostico::EstadoRef {
        io: &io_linea0,
        palette: &paleta,
        vram: &vram,
        oam: &oam,
    };
    // Primeras 80 líneas con BG0VOFS = 0
    for l in 0..80 {
        lineas[l] = linea0_ref;
    }

    // nativa y sw_fin son idénticos porque la GPU dibuja con el estado de fin de cuadro
    let nativa = vec![20u8; 240 * 160 * 3];
    let sw_fin = nativa.clone();
    // sw_linea difiere porque las primeras 80 líneas se dibujaron con scroll vertical distinto
    let mut sw_linea = nativa.clone();
    sw_linea[0] = 50;

    let diag = diagnostico::diagnosticar(&nativa, &sw_fin, &sw_linea, fin, &lineas, |_, _| 0);

    assert_eq!(diag.dif_fin, 0, "nativa y sw_fin deben coincidir con 0 diferencias");
    assert!(diag.dif_linea > 0, "sw_linea tiene diferencias");
    assert_eq!(diag.causa, "estado_por_linea", "la causa debe ser mutación entre líneas, no shader");
    assert!(diag.registros.contains(&"BG0VOFS"), "debe identificar que BG0VOFS cambió dentro del cuadro");
}

#[test]
fn test_filtro_agrupa_transicion_gradual_pero_captura_fallo_fin() {
    let mut filtro = diagnostico::FiltroFirmas::default();

    let sw = vec![100u8; 240 * 160 * 3];
    let _nat_igual = sw.clone();
    let mut nat_dif = sw.clone();
    nat_dif[0] = 200;

    // Tres cuadros donde sw_fin == nativa (0 dif fin), pero sw_linea difiere con caja que crece gradualmente
    // Cuadro 1: caja (10, 10, 40, 40)
    let dec1 = diagnostico::evaluar_cuadro_en_vivo(
        1,
        0, // dif_fin
        100, // dif_linea
        (10, 10, 40, 40),
        0x9F40,
        0x3F40,
        false,
        &mut filtro,
    );
    assert_eq!(dec1, Some("linea".to_string()), "primer cuadro de transición se captura");

    // Cuadro 2: caja (10, 10, 42, 41) dentro del mismo bloque cuantizado de 32 px
    let dec2 = diagnostico::evaluar_cuadro_en_vivo(
        2,
        0,
        110,
        (10, 10, 42, 41),
        0x9F40,
        0x3F40,
        false,
        &mut filtro,
    );
    assert!(dec2.is_none(), "segundo cuadro de transición se agrupa y omite");

    // Cuadro 3: caja (10, 10, 45, 43) aún en el mismo bloque
    let dec3 = diagnostico::evaluar_cuadro_en_vivo(
        3,
        0,
        120,
        (10, 10, 45, 43),
        0x9F40,
        0x3F40,
        false,
        &mut filtro,
    );
    assert!(dec3.is_none(), "tercer cuadro de transición se agrupa y omite");

    // Cuadro 4: aparece un fallo de fin de cuadro (dif_fin > 0)
    let dec4 = diagnostico::evaluar_cuadro_en_vivo(
        4,
        50, // dif_fin > 0
        120,
        (10, 10, 45, 43),
        0x9F40,
        0x3F40,
        false,
        &mut filtro,
    );
    assert_eq!(dec4, Some("fin".to_string()), "fallo de fin de cuadro se captura inmediatamente");
}
