//! Pruebas de integración de la PPU nativa.
//!
//! Sin ROM ni GPU: comprueban que el **shader y el código Rust no se
//! desincronicen**. Un desajuste de tamaños o de números de binding no da
//! ningún error al compilar: solo produciría basura en el dispositivo, que es
//! exactamente el fallo que el shim anterior arrastró.
//!
//! Además, los **criterios de aceptación** de la fase 1
//! (`docs/propuestas/2026-10-19_ppu-nativa-fase-1.md`):
//!
//! - Los dos criterios de la unidad de mezcla son puros y no dependen de nada.
//! - Los dos criterios que comparan píxeles leen el informe que escribe
//!   `examples/ppu_nativa_compara.rs`; se omiten si falta `FE_ROM`.
//! - El criterio de regresión (el motor base y el shim no cambian) compara
//!   hashes contra la línea de base de `tests/datos/ppu-nativa-hashes-base.txt`.

use blazing_recomp::ppu_nativa::{
    alpha_blend, brighten, darken, CapaFondoGpu, ConfigCapaGpu, ConfigCompGpu, SpriteNativa,
    BINDING_FILAS_CAPA, BINDING_FILAS_COMP, BINDING_PALETA, BINDING_SPRITES,
    BINDING_TEX_OBJ, BINDING_TEX_OBJ_VENTANA, BINDING_VRAM, PPU_NATIVA_WGSL,
};

/// Miembros de una estructura del shader, con su desplazamiento en bytes.
fn miembros_wgsl(modulo: &naga::Module, nombre: &str) -> Vec<(String, u32)> {
    for (_, ty) in modulo.types.iter() {
        if ty.name.as_deref() == Some(nombre) {
            if let naga::TypeInner::Struct { members, .. } = &ty.inner {
                return members
                    .iter()
                    .map(|m| (m.name.clone().unwrap_or_default(), m.offset))
                    .collect();
            }
        }
    }
    panic!("el shader no declara la estructura {nombre}");
}

/// Tamaño en bytes que el shader espera de una estructura.
fn tamano_wgsl(modulo: &naga::Module, nombre: &str) -> u32 {
    for (_, ty) in modulo.types.iter() {
        if ty.name.as_deref() == Some(nombre) {
            if let naga::TypeInner::Struct { span, .. } = &ty.inner {
                return *span;
            }
        }
    }
    panic!("el shader no declara la estructura {nombre}");
}

fn modulo() -> naga::Module {
    naga::front::wgsl::parse_str(PPU_NATIVA_WGSL).expect("el shader debe parsear")
}

/// Los desplazamientos del shader tienen que ser los mismos que los de Rust.
///
/// El búfer de Rust puede ser **mayor** que el tamaño que el shader espera
/// (el mínimo uniforme se redondea a 16 bytes), pero ningún miembro puede
/// estar en otro sitio.
#[test]
fn los_desplazamientos_de_las_estructuras_coinciden() {
    let m = modulo();

    let rust_capafondo: Vec<(&str, u32)> = vec![
        ("enabled", std::mem::offset_of!(CapaFondoGpu, enabled) as u32),
        ("priority", std::mem::offset_of!(CapaFondoGpu, priority) as u32),
        ("char_base", std::mem::offset_of!(CapaFondoGpu, char_base) as u32),
        (
            "screen_base",
            std::mem::offset_of!(CapaFondoGpu, screen_base) as u32,
        ),
        (
            "screen_size",
            std::mem::offset_of!(CapaFondoGpu, screen_size) as u32,
        ),
        ("is_8bpp", std::mem::offset_of!(CapaFondoGpu, is_8bpp) as u32),
        ("scroll_x", std::mem::offset_of!(CapaFondoGpu, scroll_x) as u32),
        ("scroll_y", std::mem::offset_of!(CapaFondoGpu, scroll_y) as u32),
        ("es_afin", std::mem::offset_of!(CapaFondoGpu, es_afin) as u32),
        (
            "afin_wrap",
            std::mem::offset_of!(CapaFondoGpu, afin_wrap) as u32,
        ),
        ("afin_pa", std::mem::offset_of!(CapaFondoGpu, afin_pa) as u32),
        ("afin_pb", std::mem::offset_of!(CapaFondoGpu, afin_pb) as u32),
        ("afin_pc", std::mem::offset_of!(CapaFondoGpu, afin_pc) as u32),
        ("afin_pd", std::mem::offset_of!(CapaFondoGpu, afin_pd) as u32),
        (
            "afin_ref_x",
            std::mem::offset_of!(CapaFondoGpu, afin_ref_x) as u32,
        ),
        (
            "afin_ref_y",
            std::mem::offset_of!(CapaFondoGpu, afin_ref_y) as u32,
        ),
    ];
    let shader_capafondo = miembros_wgsl(&m, "CapaFondo");
    assert_eq!(
        shader_capafondo,
        rust_capafondo
            .iter()
            .map(|(n, o)| (n.to_string(), *o))
            .collect::<Vec<_>>()
    );

    let rust_configcapa: Vec<(&str, u32)> = vec![
        ("modo", std::mem::offset_of!(ConfigCapaGpu, modo) as u32),
        ("_pad0", std::mem::offset_of!(ConfigCapaGpu, _pad0) as u32),
        ("_pad1", std::mem::offset_of!(ConfigCapaGpu, _pad1) as u32),
        ("_pad2", std::mem::offset_of!(ConfigCapaGpu, _pad2) as u32),
        ("capas", std::mem::offset_of!(ConfigCapaGpu, capas) as u32),
    ];
    let shader_configcapa = miembros_wgsl(&m, "ConfigCapa");
    assert_eq!(shader_configcapa.len(), rust_configcapa.len());
    for ((nombre, off), (shader_nombre, shader_off)) in
        rust_configcapa.iter().zip(shader_configcapa.iter())
    {
        assert_eq!(
            *shader_off, *off,
            "el desplazamiento de {nombre} no coincide en el shader"
        );
        assert_eq!(shader_nombre, &nombre.to_string(), "el nombre de {nombre}");
    }

    let rust_configcomp: Vec<(&str, u32)> = vec![
        ("dispcnt", std::mem::offset_of!(ConfigCompGpu, dispcnt) as u32),
        (
            "bg_prio0",
            std::mem::offset_of!(ConfigCompGpu, bg_prio0) as u32,
        ),
        (
            "bg_prio1",
            std::mem::offset_of!(ConfigCompGpu, bg_prio1) as u32,
        ),
        (
            "bg_prio2",
            std::mem::offset_of!(ConfigCompGpu, bg_prio2) as u32,
        ),
        (
            "bg_prio3",
            std::mem::offset_of!(ConfigCompGpu, bg_prio3) as u32,
        ),
        ("win0h", std::mem::offset_of!(ConfigCompGpu, win0h) as u32),
        ("win1h", std::mem::offset_of!(ConfigCompGpu, win1h) as u32),
        ("win0v", std::mem::offset_of!(ConfigCompGpu, win0v) as u32),
        ("win1v", std::mem::offset_of!(ConfigCompGpu, win1v) as u32),
        ("winin", std::mem::offset_of!(ConfigCompGpu, winin) as u32),
        ("winout", std::mem::offset_of!(ConfigCompGpu, winout) as u32),
        ("bldcnt", std::mem::offset_of!(ConfigCompGpu, bldcnt) as u32),
        (
            "bldalpha",
            std::mem::offset_of!(ConfigCompGpu, bldalpha) as u32,
        ),
        ("bldy", std::mem::offset_of!(ConfigCompGpu, bldy) as u32),
        ("_pad0", std::mem::offset_of!(ConfigCompGpu, _pad0) as u32),
        ("_pad1", std::mem::offset_of!(ConfigCompGpu, _pad1) as u32),
    ];
    let shader_configcomp = miembros_wgsl(&m, "ConfigComp");
    assert_eq!(shader_configcomp.len(), rust_configcomp.len());
    for ((nombre, off), (shader_nombre, shader_off)) in
        rust_configcomp.iter().zip(shader_configcomp.iter())
    {
        assert_eq!(
            *shader_off, *off,
            "el desplazamiento de {nombre} no coincide en el shader"
        );
        assert_eq!(shader_nombre, &nombre.to_string(), "el nombre de {nombre}");
    }

    let rust_sprite: Vec<(&str, u32)> = vec![
        ("pos_x", std::mem::offset_of!(SpriteNativa, pos_x) as u32),
        ("pos_y", std::mem::offset_of!(SpriteNativa, pos_y) as u32),
        ("caja_x", std::mem::offset_of!(SpriteNativa, caja_x) as u32),
        ("caja_y", std::mem::offset_of!(SpriteNativa, caja_y) as u32),
        ("tex_x", std::mem::offset_of!(SpriteNativa, tex_x) as u32),
        ("tex_y", std::mem::offset_of!(SpriteNativa, tex_y) as u32),
        (
            "base_tile",
            std::mem::offset_of!(SpriteNativa, base_tile) as u32,
        ),
        (
            "palette_idx",
            std::mem::offset_of!(SpriteNativa, palette_idx) as u32,
        ),
        ("flags", std::mem::offset_of!(SpriteNativa, flags) as u32),
        (
            "prioridad",
            std::mem::offset_of!(SpriteNativa, prioridad) as u32,
        ),
        ("afin_a", std::mem::offset_of!(SpriteNativa, afin_a) as u32),
        ("afin_b", std::mem::offset_of!(SpriteNativa, afin_b) as u32),
        ("afin_c", std::mem::offset_of!(SpriteNativa, afin_c) as u32),
        ("afin_d", std::mem::offset_of!(SpriteNativa, afin_d) as u32),
        (
            "oam_index",
            std::mem::offset_of!(SpriteNativa, oam_index) as u32,
        ),
    ];
    let shader_sprite = miembros_wgsl(&m, "SpriteNativa");
    assert_eq!(shader_sprite.len(), rust_sprite.len());
    for ((nombre, off), (shader_nombre, shader_off)) in
        rust_sprite.iter().zip(shader_sprite.iter())
    {
        assert_eq!(
            *shader_off, *off,
            "el desplazamiento de {nombre} no coincide en el shader"
        );
        assert_eq!(shader_nombre, &nombre.to_string(), "el nombre de {nombre}");
    }
}

/// El búfer de Rust tiene que ser al menos tan grande como lo que el shader
/// espera leer de él.
#[test]
fn los_buffers_de_rust_caben_lo_que_el_shader_lee() {
    let m = modulo();
    assert!(
        std::mem::size_of::<CapaFondoGpu>() as u32 >= tamano_wgsl(&m, "CapaFondo"),
        "CapaFondoGpu es más pequeño que la del shader"
    );
    assert!(
        std::mem::size_of::<ConfigCapaGpu>() as u32 >= tamano_wgsl(&m, "ConfigCapa"),
        "ConfigCapaGpu es más pequeño que la del shader"
    );
    assert!(
        std::mem::size_of::<ConfigCompGpu>() as u32 >= tamano_wgsl(&m, "ConfigComp"),
        "ConfigCompGpu es más pequeño que la del shader"
    );
    assert!(
        std::mem::size_of::<SpriteNativa>() as u32 >= tamano_wgsl(&m, "SpriteNativa"),
        "SpriteNativa es más pequeña que la del shader"
    );
}

/// Los números de binding del shader tienen que ser los que Rust usa al
/// montar los grupos de enlaces.
#[test]
fn los_bindings_del_shader_coinciden_con_las_constantes_de_rust() {
    let m = modulo();
    let esperado: Vec<(&str, u32)> = vec![
        ("vram", BINDING_VRAM),
        ("paleta", BINDING_PALETA),
        ("filas_capa", BINDING_FILAS_CAPA),
        ("sprites", BINDING_SPRITES),
        ("filas_comp", BINDING_FILAS_COMP),
        ("tex_obj", BINDING_TEX_OBJ),
        ("tex_obj_ventana", BINDING_TEX_OBJ_VENTANA),
    ];
    for (nombre, binding) in &esperado {
        let var = m
            .global_variables
            .iter()
            .find(|(_, v)| v.name.as_deref() == Some(*nombre))
            .unwrap_or_else(|| panic!("el shader no declara la variable {nombre}"));
        let recurso = var
            .1
            .binding
            .unwrap_or_else(|| panic!("{nombre} no tiene binding"));
        assert_eq!(recurso.group, 0, "{nombre} debe estar en el grupo 0");
        assert_eq!(recurso.binding, *binding, "binding de {nombre}");
    }
    // Ni una variable más: un recurso de sobra sería un error de validación.
    assert_eq!(
        m.global_variables.len(),
        esperado.len(),
        "el shader declara más variables de las previstas"
    );
}

/// El shader tiene que exponer los puntos de entrada que usa el pipeline.
#[test]
fn el_shader_expone_los_puntos_de_entrada_del_pipeline() {
    let m = modulo();
    for punto in [
        "vs_quad",
        "vs_obj",
        "fs_obj",
        "fs_obj_ventana",
        "fs_comp",
    ] {
        assert!(
            m.entry_points.iter().any(|e| e.name == punto),
            "falta el punto de entrada {punto}"
        );
    }
}

/// El shader tiene que indexar la paleta por línea vertical (`512u`) en fondos y sprites.
#[test]
fn test_shader_indexa_paleta_por_linea_vertical() {
    let m = modulo();
    let mut validator = naga::valid::Validator::new(
        naga::valid::ValidationFlags::all(),
        naga::valid::Capabilities::all(),
    );
    validator
        .validate(&m)
        .expect("el shader con paleta vertical 2D debe validar con naga");

    assert!(
        PPU_NATIVA_WGSL.contains("paleta[py * 512u + idx]"),
        "los fondos afines y 8bpp deben indexar la paleta por línea"
    );
    assert!(
        PPU_NATIVA_WGSL.contains("paleta[py * 512u + pal_base + nib]"),
        "los fondos 4bpp deben indexar la paleta por línea"
    );
    assert!(
        PPU_NATIVA_WGSL.contains("paleta[upy * 512u + pal]"),
        "los sprites deben indexar la paleta por línea"
    );
    assert!(
        PPU_NATIVA_WGSL.contains("paleta[upy * 512u]"),
        "el color de fondo (backdrop) debe indexar la paleta por línea"
    );
}

// ===========================================================================
// Criterios de aceptación de la fase 1
// ===========================================================================

/// El criterio 1: la mezcla alfa con coeficientes intermedios usa el carril
/// de verde de 6 bits.
///
/// Con EVA = EVB = 8 sobre verde 15 y verde 16, el verde de 5 bits tiene que
/// quedar en 15 (la aritmética de 5 bits daría 16). Con EVA = 4 y EVB = 12
/// sobre el mismo par, tiene que quedar en 16.
///
/// El mismo redondeo a media hacia arriba (`+8 >> 4`) se aplica al rojo y al
/// azul: con EVA = EVB = 8 sobre rojo 15 y rojo 16, el resultado es 16 (el
/// valor real es 15.5 y la referencia redondea hacia arriba; truncar daría 15).
/// Ningún punto de entrada de vértices lee la lista de sprites.
///
/// No es una búsqueda de texto: `naga` analiza el módulo y dice qué globales
/// lee cada punto de entrada. Ver la lista desde el vértice exigiría la
/// capacidad `DownlevelFlags::VERTEX_STORAGE`, que no todos los dispositivos
/// dan: donde falta, la creación del layout aborta y la app muere.
#[test]
fn ningun_punto_de_entrada_de_vertices_lee_la_lista_de_sprites() {
    let module = naga::front::wgsl::parse_str(PPU_NATIVA_WGSL).expect("el shader debe parsear");
    let mut validator = naga::valid::Validator::new(
        naga::valid::ValidationFlags::all(),
        naga::valid::Capabilities::all(),
    );
    let info = validator
        .validate(&module)
        .expect("el shader debe validar con naga");

    let (handle, global) = module
        .global_variables
        .iter()
        .find(|(_, g)| {
            matches!(g.binding, Some(b) if b.group == 0 && b.binding == BINDING_SPRITES)
        })
        .expect("el shader debe declarar la lista de sprites");
    assert_eq!(global.name.as_deref(), Some("sprites"));

    for (indice, punto) in module.entry_points.iter().enumerate() {
        if punto.stage != naga::ShaderStage::Vertex {
            continue;
        }
        let uso = info.get_entry_point(indice)[handle];
        assert!(
            !uso.contains(naga::valid::GlobalUse::READ),
            "el punto de entrada `{}` lee la lista de sprites desde la etapa de vértices",
            punto.name
        );
    }

    // Y la geometría llega al vértice por atributos de instancia: el quad en
    // la ubicación 0, la caja del sprite en la 1 y su clave de prioridad en la 2.
    let vs_obj = module
        .entry_points
        .iter()
        .find(|p| p.name == "vs_obj")
        .expect("el shader debe tener el punto de entrada `vs_obj`");
    let ubicaciones: Vec<u32> = vs_obj
        .function
        .arguments
        .iter()
        .filter_map(|arg| match arg.binding {
            Some(naga::Binding::Location { location, .. }) => Some(location),
            _ => None,
        })
        .collect();
    assert_eq!(
        ubicaciones,
        vec![0, 1, 2],
        "`vs_obj` debe recibir el quad y los dos atributos de instancia"
    );

    let tipo_de = |localacion: u32| {
        vs_obj.function.arguments.iter().find_map(|arg| {
            match arg.binding {
                Some(naga::Binding::Location { location, .. }) if location == localacion => {}
                _ => return None,
            }
            match &module.types[arg.ty].inner {
                naga::TypeInner::Vector { size, scalar } => Some((*size, *scalar)),
                _ => None,
            }
        })
    };
    assert_eq!(
        tipo_de(1),
        Some((naga::VectorSize::Quad, naga::Scalar::F32)),
        "la ubicación 1 lleva la caja del sprite"
    );
    assert_eq!(
        tipo_de(2),
        Some((naga::VectorSize::Bi, naga::Scalar::U32)),
        "la ubicación 2 lleva la clave de prioridad"
    );
}

#[test]
fn test_alfa_coeficientes_intermedios() {
    // Verde de 5 bits en las posiciones 5-9.
    let verde15 = 15u16 << 5;
    let verde16 = 16u16 << 5;

    let a_8_8 = alpha_blend(verde15, verde16, 8, 8);
    assert_eq!(
        (a_8_8 >> 5) & 31,
        15,
        "con EVA = EVB = 8 el verde tiene que quedar en 15, no en 16"
    );

    let a_4_12 = alpha_blend(verde15, verde16, 4, 12);
    assert_eq!(
        (a_4_12 >> 5) & 31,
        16,
        "con EVA = 4 y EVB = 12 el verde tiene que quedar en 16"
    );

    // Rojo: el mismo redondeo a media hacia arriba que la referencia.
    let r_8_8 = alpha_blend(15, 16, 8, 8);
    assert_eq!(
        r_8_8 & 31,
        16,
        "con EVA = EVB = 8 el rojo tiene que redondear a media hacia arriba: 16"
    );
    // Azul: igual que el rojo.
    let b_8_8 = alpha_blend(15 << 10, 16 << 10, 8, 8);
    assert_eq!(
        (b_8_8 >> 10) & 31,
        16,
        "con EVA = EVB = 8 el azul tiene que redondear a media hacia arriba: 16"
    );
}

/// El criterio 2: los extremos de la unidad de mezcla saturan y son idénticos.
///
/// Con EVA = EVB = 16, la mezcla de blanco con blanco da blanco; con EVY = 16,
/// aclarar el negro da blanco y oscurecer el blanco da negro.
#[test]
fn test_extremos_de_la_mezcla() {
    // EVA = EVB = 16 sobre blanco: el resultado satura en blanco.
    assert_eq!(alpha_blend(0x7FFF, 0x7FFF, 16, 16), 0x7FFF);
    // EVA = EVB = 16 sobre rojo: el rojo se conserva (coeficiente pleno).
    assert_eq!(alpha_blend(0x001F, 0x001F, 16, 16), 0x001F);
    // Aclarar el negro con EVY = 16 da blanco.
    assert_eq!(brighten(0x0000, 16), 0x7FFF);
    // Oscurecer el blanco con EVY = 16 da negro.
    assert_eq!(darken(0x7FFF, 16), 0x0000);
}

/// Aclarar colores no-cero con coeficientes intermedios: la fórmula
/// `x + (((31 - x) * evy + 8) >> 4)` con precedencia explícita.
#[test]
fn test_brighten_wgsl_parentesis_y_no_cero_coinciden() {
    // Para canal = 10 y evy = 8: 10 + (((31 - 10) * 8 + 8) >> 4) = 10 + (176 >> 4) = 10 + 11 = 21.
    // Sin paréntesis adecuados en WGSL, la precedencia de operadores causaba underflows o desbordamientos.
    let color_10 = 10u16 | (10u16 << 5) | (10u16 << 10);
    let resultado = brighten(color_10, 8);
    let r = resultado & 31;
    let g = (resultado >> 5) & 31;
    let b = (resultado >> 10) & 31;
    assert_eq!(r, 21, "el canal rojo debe ser 21");
    assert_eq!(g, 21, "el canal verde debe ser 21");
    assert_eq!(b, 21, "el canal azul debe ser 21");
}

/// Lee el informe que escribe `examples/ppu_nativa_compara.rs`.
fn leer_informe(cuadro: u32) -> Option<String> {
    let ruta = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("volcados")
        .join(format!("ppu-nativa-{cuadro}.txt"));
    std::fs::read_to_string(ruta).ok()
}

/// Extrae un número de una línea del informe que lo contiene.
fn numero_de(texto: &str, etiqueta: &str) -> Option<u64> {
    let linea = texto
        .lines()
        .find(|l| l.contains(etiqueta))?;
    let resto = linea.split(etiqueta).nth(1)?;
    let digitos: String = resto
        .chars()
        .skip_while(|c| !c.is_ascii_digit())
        .take_while(|c| c.is_ascii_digit())
        .collect();
    digitos.parse().ok()
}

/// El criterio 3: las líneas sin afines del cuadro 5300 coinciden con la PPU.
///
/// El cuadro 5300 tiene escenas con mezcla alfa y sin sprites afines. La
/// comparación justa es contra la referencia alimentada con el mismo estado de
/// fin de cuadro que recibe la GPU (el runtime toma VRAM, OAM, paleta e I/O
/// una sola vez por cuadro).
#[test]
fn test_lineas_sin_afines_5300_coinciden() {
    if std::env::var("FE_ROM").is_err() {
        eprintln!("omitida: falta FE_ROM");
        return;
    }
    let Some(texto) = leer_informe(5300) else {
        eprintln!(
            "omitida: falta volcados/ppu-nativa-5300.txt (correr \
             examples/ppu_nativa_compara con el cuadro 5300)"
        );
        return;
    };
    // El control del arnés: si falla, el informe no vale.
    assert_eq!(
        numero_de(&texto, "control reconstruccion frente a framebuffer del emulador:"),
        Some(0),
        "el control del arnés debe dar 0 diferencias"
    );
    // La PPU nativa tiene que coincidir con la referencia de fin de cuadro.
    assert_eq!(
        numero_de(&texto, "diferencias frente a la referencia de estado de fin de cuadro:"),
        Some(0),
        "la PPU nativa debe coincidir con la referencia de fin de cuadro"
    );
    // Y el veredicto del arnés sobre las líneas sin afines.
    assert!(
        texto.contains("criterio de la fase 1 (0 diferencias en las lineas sin afines): CUMPLIDO"),
        "el arnés no da el criterio por cumplido:\n{}",
        texto.lines().take(8).collect::<Vec<_>>().join("\n")
    );
    // La diferencia entre las dos referencias tiene que explicar **toda** la
    // diferencia contra la referencia por línea: si quedara residuo, sería un
    // fallo de la lógica de dibujo, no del estado dentro del cuadro.
    let por_linea = numero_de(
        &texto,
        "referencia de estado por linea (fuera del alcance de esta fase):",
    )
    .expect("el informe debe traer el total de la referencia por línea");
    let entre_referencias = numero_de(
        &texto,
        "diferencias entre las dos referencias (solo efecto del estado dentro del cuadro):",
    )
    .expect("el informe debe traer la diferencia entre las dos referencias");
    assert_eq!(
        por_linea, entre_referencias,
        "todo el residuo contra la referencia por línea tiene que venir del \
         estado que cambia dentro del cuadro"
    );
}

/// El criterio 4: un cuadro sin efectos ni ventanas no cambia.
///
/// El cuadro 7500 de la misma secuencia no activa ningún efecto de color ni
/// ventana, y no cambia nada dentro del cuadro: tiene que dar 0 diferencias
/// de 38400.
#[test]
fn test_cuadro_7500_sin_efectos_coincide() {
    if std::env::var("FE_ROM").is_err() {
        eprintln!("omitida: falta FE_ROM");
        return;
    }
    let Some(texto) = leer_informe(7500) else {
        eprintln!(
            "omitida: falta volcados/ppu-nativa-7500.txt (correr \
             examples/ppu_nativa_compara con el cuadro 7500)"
        );
        return;
    };
    assert_eq!(
        numero_de(&texto, "control reconstruccion frente a framebuffer del emulador:"),
        Some(0),
        "el control del arnés debe dar 0 diferencias"
    );
    // 0 diferencias de 38400 contra la referencia de fin de cuadro...
    assert_eq!(
        numero_de(&texto, "diferencias frente a la referencia de estado de fin de cuadro:"),
        Some(0),
        "la PPU nativa debe coincidir con la referencia de fin de cuadro"
    );
    // ...y también contra la referencia por línea, porque este cuadro no
    // cambia nada dentro de él.
    assert_eq!(
        numero_de(
            &texto,
            "referencia de estado por linea (fuera del alcance de esta fase):"
        ),
        Some(0),
        "el cuadro 7500 no cambia dentro del cuadro: las dos referencias tienen \
         que coincidir"
    );
}

/// El criterio del fondo afín: el cuadro 900 de `escena3` coincide con la
/// referencia de fin de cuadro.
///
/// Es el cuadro con el fondo afín BG2 activado (`DISPCNT = 0x1F01`, bit 10) y
/// con la imagen deformada: es el caso que el juego muestra en los menús. La
/// comparación justa es contra la referencia alimentada con el mismo estado de
/// fin de cuadro que recibe la GPU.
#[test]
fn test_cuadro_900_escena3_fondo_afin_coincide() {
    if std::env::var("FE_ROM").is_err() {
        eprintln!("omitida: falta FE_ROM");
        return;
    }
    let Some(texto) = leer_informe(900) else {
        eprintln!(
            "omitida: falta volcados/ppu-nativa-900.txt (correr \
             examples/ppu_nativa_compara con el cuadro 900)"
        );
        return;
    };
    // El control del arnés: si falla, el informe no vale.
    assert_eq!(
        numero_de(&texto, "control reconstruccion frente a framebuffer del emulador:"),
        Some(0),
        "el control del arnés debe dar 0 diferencias"
    );
    // La PPU nativa tiene que coincidir con la referencia de fin de cuadro: el
    // fondo afín se dibuja.
    assert_eq!(
        numero_de(&texto, "diferencias frente a la referencia de estado de fin de cuadro:"),
        Some(0),
        "la PPU nativa debe coincidir con la referencia de fin de cuadro"
    );
    // Y el veredicto del arnés sobre las líneas sin afines.
    assert!(
        texto.contains("criterio de la fase 1 (0 diferencias en las lineas sin afines): CUMPLIDO"),
        "el arnés no da el criterio por cumplido:\n{}",
        texto.lines().take(8).collect::<Vec<_>>().join("\n")
    );
}

/// El criterio del arnés del fondo afín: los casos límite coinciden.
///
/// El arnés `examples/ppu_nativa_afin.rs` monta un estado sintético (mapa
/// visible, wrap apagado con la referencia fuera del mapa, wrap activado con la
/// referencia negativa y los cuatro tamaños de mapa) y lo compara con
/// `gba_core::ppu::render_scanline` sobre el mismo estado.
#[test]
fn test_fondo_afin_casos_limite_coinciden() {
    let ruta = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("volcados")
        .join("ppu-nativa-afin.txt");
    let Ok(texto) = std::fs::read_to_string(&ruta) else {
        eprintln!(
            "omitida: falta volcados/ppu-nativa-afin.txt (correr \
             examples/ppu_nativa_afin)"
        );
        return;
    };
    for caso in [
        "mapa visible (32 tiles)",
        "wrap apagado con la referencia fuera del mapa (32 tiles)",
        "wrap activado con la referencia negativa (32 tiles)",
        "mapa de 16 tiles",
        "mapa de 32 tiles",
        "mapa de 64 tiles",
        "mapa de 128 tiles",
    ] {
        let linea = texto
            .lines()
            .find(|l| l.starts_with(&format!("caso {caso}:")))
            .unwrap_or_else(|| panic!("el informe no trae el caso '{caso}'"));
        assert_eq!(
            diferencias_de(linea),
            Some(0),
            "el caso '{caso}' no coincide con la referencia:\n{linea}"
        );
    }
    // El wrap tiene que dar la vuelta al mapa, no recortar: el píxel (0, 0), que
    // está fuera del mapa, se dibuja en la nativa y en la referencia.
    let linea = texto
        .lines()
        .find(|l| l.starts_with("caso wrap activado con la referencia negativa (32 tiles):"))
        .expect("el informe debe traer el caso del wrap");
    assert!(
        linea.contains("pixel fuera del mapa dibujado: si"),
        "el wrap tiene que dibujar el píxel fuera del mapa:\n{linea}"
    );
}

/// Número de diferencias de la línea de un caso del informe del arnés.
fn diferencias_de(linea: &str) -> Option<u64> {
    let resto = linea.split_once(": ")?.1;
    let digitos: String = resto.chars().take_while(|c| c.is_ascii_digit()).collect();
    digitos.parse().ok()
}

/// El criterio 5: el motor base y el shim actual no cambian.
///
/// El shim queda congelado durante la fase: su borrado (paso 8) solo puede
/// ocurrir cuando todos los demás criterios se hayan cumplido. El motor base
/// se comprueba contra el rev fijado `de4edf59` de `gba-recomp`.
#[test]
fn test_el_motor_base_y_el_shim_no_cambian() {
    let raiz = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let linea_base = raiz.join("tests/datos/ppu-nativa-hashes-base.txt");
    let texto = std::fs::read_to_string(&linea_base).unwrap_or_else(|e| {
        panic!("no se pudo leer {}: {e}", linea_base.display())
    });

    // Localiza el checkout del rev fijado dentro del registro de Cargo.
    let cargo_home = std::env::var("CARGO_HOME").unwrap_or_else(|_| {
        let home = std::env::var("HOME").unwrap_or_default();
        format!("{home}/.cargo")
    });
    let rev_corto = "de4edf5";
    let base_upstream = std::path::PathBuf::from(&cargo_home)
        .join("git/checkouts")
        .join("gba-recomp-29c6b0a11a692bcd")
        .join(rev_corto);

    let mut comprobados = 0;
    for linea in texto.lines() {
        let linea = linea.trim();
        if linea.is_empty() || linea.starts_with('#') {
            continue;
        }
        let Some((id, hash)) = linea.split_once(' ') else {
            panic!("línea mal formada en la línea de base: {linea}");
        };
        let (id, hash) = (id.trim(), hash.trim());

        let ruta = if id.starts_with("src/") {
            raiz.join(id)
        } else if let Some(rest) = id.strip_prefix("gba-core:") {
            // gba-core:<rev>:<ruta dentro del motor>
            let mut partes = rest.splitn(2, ':');
            let rev = partes.next().unwrap_or_default();
            let ruta_rel = partes.next().unwrap_or_default();
            assert_eq!(
                rev, rev_corto,
                "la línea de base apunta a un rev distinto del fijado"
            );
            base_upstream.join(ruta_rel)
        } else {
            panic!("identificador desconocido en la línea de base: {id}");
        };

        if !ruta.exists() {
            // El checkout del upstream no está en esta máquina: no se puede
            // comprobar ese hash, pero los del repositorio sí.
            eprintln!("omitido (no está en esta máquina): {id}");
            continue;
        }
        let hash_calculado = blazing_recomp::storage::compute_sha256(
            &std::fs::read(&ruta).unwrap_or_else(|e| panic!("no se pudo leer {id}: {e}")),
        );
        assert_eq!(
            hash_calculado, hash,
            "el hash de {id} no es el de la línea de base: ha cambiado"
        );
        comprobados += 1;
    }
    assert!(
        comprobados >= 3,
        "solo se comprobaron {comprobados} ficheros: el shim tiene que estar \
         entre ellos"
    );
}
