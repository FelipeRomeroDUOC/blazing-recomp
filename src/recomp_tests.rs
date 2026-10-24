use super::*;

/// Serializa los tests que escriben en el acumulador global de hotspots.
fn bloquear_acumuladores() -> std::sync::MutexGuard<'static, ()> {
    LOCK_ACUMULADORES.lock().unwrap_or_else(|e| e.into_inner())
}

extern "C" fn dummy_block_fn(_api: *const gba_core::capi::RtApi, mptr: *mut std::ffi::c_void) -> u32 {
    let mach = unsafe { &mut *(mptr as *mut Machine) };
    mach.cpu.regs[15] = 0x0800_0100;
    mach.bus.tick(2);
    0x0800_0100
}

#[test]
fn test_block_table_indexed_dispatch() {
    let block = RcgBlock {
        key: 0x0800_00C0 | 1, // ROM Thumb
        func: dummy_block_fn,
    };
    let blocks = [block];
    let table = BlockTable::from_blocks(&blocks);

    assert_eq!(table.len(), 1);
    assert!(!table.is_empty());

    // Dirección mapeada con Thumb bit debe resolver en O(1)
    let found = table.get(0x0800_00C0 | 1);
    assert!(found.is_some());

    // Dirección no mapeada debe retornar None
    assert!(table.get(0x0800_00D0 | 1).is_none());
    assert!(table.get(0x0300_0000).is_none());
}

#[test]
fn test_run_frame_native_fallback_on_unmapped() {
    let _guard = bloquear_acumuladores();
    let mut rom = vec![0u8; 1024 * 1024];
    rom[0] = 0x2E; // B 0x080000C0
    rom[3] = 0xEA;
    let mut machine = Machine::new(rom);
    let table = BlockTable::empty();

    let initial_clock = machine.bus.clock;
    let (native, fallback) = run_frame_native(&mut machine, &table, 100);

    assert_eq!(native, 0, "con tabla vacía no debe haber bloques nativos");
    assert!(fallback > 0, "debe avanzar mediante intérprete de fallback");
    assert!(machine.bus.clock > initial_clock, "el reloj debe avanzar");
}

#[test]
fn test_block_table_iwram_and_bios_regions() {
    extern "C" fn dummy_iwram(_api: *const gba_core::capi::RtApi, _m: *mut std::ffi::c_void) -> u32 { 0 }
    extern "C" fn dummy_bios(_api: *const gba_core::capi::RtApi, _m: *mut std::ffi::c_void) -> u32 { 0 }

    let blocks = [
        RcgBlock { key: 0x0300_1000 | 1, func: dummy_iwram },
        RcgBlock { key: 0x0000_0080, func: dummy_bios },
    ];
    let table = BlockTable::from_blocks(&blocks);

    assert_eq!(table.len(), 2);
    assert!(table.get(0x0300_1000 | 1).is_some());
    assert!(table.get(0x0000_0080).is_some());
    assert!(table.get(0x0300_2000).is_none());
}

#[test]
fn test_block_table_despacha_rutinas_de_combate_en_iwram() {
    extern "C" fn dummy_combat(_api: *const gba_core::capi::RtApi, _m: *mut std::ffi::c_void) -> u32 { 0 }

    // Clúster crítico de subrutinas de combate en IWRAM (ARM32, bit 0 = 0)
    let combat_addrs = [
        0x0300_3478, // InterpCalc
        0x0300_3568, // DispatchLoop
        0x0300_35C8, // Case4 Branch
        0x0300_35CC, // Case5 Branch
        0x0300_35D0, // Case3 Branch
        0x0300_35D4, // Case2 Branch
        0x0300_35D8, // Case6 Branch
        0x0300_35DC, // Case1 Body
        0x0300_3620, // Case2 Body
        0x0300_3654, // Case3 Body
        0x0300_3688, // Case4 Body
        0x0300_36BC, // Case5 Body
        0x0300_36CC, // Case5 Step 2
        0x0300_36EC, // Epilogue
        0x0300_3708, // Case6 Body
    ];

    let blocks: Vec<RcgBlock> = combat_addrs
        .iter()
        .map(|&addr| RcgBlock {
            key: addr, // ARM mode (sin bit Thumb)
            func: dummy_combat,
        })
        .collect();

    let table = BlockTable::from_blocks(&blocks);
    assert_eq!(table.len(), combat_addrs.len());

    for &addr in &combat_addrs {
        let entry = table.get(addr);
        assert!(
            entry.is_some(),
            "la dirección de combate en IWRAM 0x{:08X} debe resolver nativamente en O(1)",
            addr
        );
    }

    // Comprobar que direcciones adyacentes no mapeadas no colisionen
    assert!(table.get(0x0300_35C4).is_none());
    assert!(table.get(0x0300_35E0).is_none());
}

// Antes existía aquí `test_run_frame_native_preserves_real_bios_stepping`, que exigía
// `native == 0` en la BIOS real. Su contrato se retiró (propuesta
// `2026-10-12_ejecucion-nativa-de-bios-real`): el motor en el rev fijado mantiene el flag
// `pc_in_bios` en el despacho y ejecuta la BIOS con sus bloques, sin desviarla al intérprete.
// El invariante que sí importa (el flag) tiene su propio test, y la aserción invertida
// (`native == 1`) es la del test normal.
#[test]
fn test_run_frame_native_despacha_bloque_aot_de_bios_en_modo_bios_real() {
    let _guard = bloquear_acumuladores();
    static mut BIOS_CALLED: bool = false;
    extern "C" fn bios_block(_api: *const gba_core::capi::RtApi, _m: *mut std::ffi::c_void) -> u32 {
        unsafe { BIOS_CALLED = true; }
        0
    }

    let blocks = [RcgBlock { key: 0x0000_0080, func: bios_block }];
    let table = BlockTable::from_blocks(&blocks);

    let rom = vec![0u8; 1024];
    let bios = vec![0u8; 16384];
    let mut machine = Machine::new_with_bios(rom, &bios);
    machine.cpu.regs[15] = 0x0000_0080;

    let (native, fallback) = run_frame_native(&mut machine, &table, 1);

    assert!(unsafe { BIOS_CALLED }, "el bloque AOT de la BIOS debe ejecutarse");
    assert_eq!(native, 1, "un despacho nativo");
    assert_eq!(fallback, 0, "sin pasos de intérprete");
}

/// La red de seguridad del intérprete sigue disponible dentro de la BIOS: una dirección sin
/// bloque cae al intérprete y queda contada en la región `bios` de la telemetría.
#[test]
fn test_run_frame_native_direccion_de_bios_sin_bloque_cae_al_interprete() {
    let _guard = bloquear_acumuladores();
    extern "C" fn bios_block(_api: *const gba_core::capi::RtApi, _m: *mut std::ffi::c_void) -> u32 {
        0
    }

    let blocks = [RcgBlock { key: 0x0000_0080, func: bios_block }];
    let table = BlockTable::from_blocks(&blocks);

    let rom = vec![0u8; 1024];
    let bios = vec![0u8; 16384];
    let mut machine = Machine::new_with_bios(rom, &bios);
    machine.cpu.regs[15] = 0x0000_2000;

    let antes = region_counters().interprete[Region::Bios.index()];
    let (native, fallback) = run_frame_native(&mut machine, &table, 1);
    let despues = region_counters().interprete[Region::Bios.index()];

    assert_eq!(native, 0, "no hay bloque en esa dirección");
    assert_eq!(fallback, 1, "debe avanzar con el intérprete");
    assert_eq!(despues - antes, 1, "y quedar contada en la región bios");
}

/// Contrato del motor: los bloques nativos no pasan por `note_fetch`, así que el flag de
/// "estoy ejecutando dentro de la BIOS" se mantiene en el despacho (las lecturas de la región 0
/// devuelven bytes reales solo mientras está puesto).
#[test]
fn test_run_frame_native_mantiene_pc_in_bios_en_el_despacho() {
    let _guard = bloquear_acumuladores();
    extern "C" fn bios_block(_api: *const gba_core::capi::RtApi, _m: *mut std::ffi::c_void) -> u32 {
        0
    }
    extern "C" fn rom_block(_api: *const gba_core::capi::RtApi, _m: *mut std::ffi::c_void) -> u32 {
        0
    }

    let blocks = [
        RcgBlock { key: 0x0000_0080, func: bios_block },
        RcgBlock { key: 0x0800_0100, func: rom_block },
    ];
    let table = BlockTable::from_blocks(&blocks);
    let bios_img = vec![0u8; 16384];

    let mut en_bios = Machine::new_with_bios(vec![0u8; 1024], &bios_img);
    en_bios.cpu.regs[15] = 0x0000_0080;
    let _ = run_frame_native(&mut en_bios, &table, 1);
    assert!(en_bios.bus.pc_in_bios, "ejecutando dentro de la BIOS");

    let mut en_rom = Machine::new_with_bios(vec![0u8; 1024 * 1024], &bios_img);
    en_rom.cpu.regs[15] = 0x0800_0100;
    let _ = run_frame_native(&mut en_rom, &table, 1);
    assert!(!en_rom.bus.pc_in_bios, "ejecutando fuera de la BIOS");
}

#[test]
fn test_recomp_canonical_path_derivation() {
    let self_path = "/data/app/~~random/com.blazingrecomp.game/lib/arm64/libblazing_recomp.so";
    let sibling = resolve_sibling_library_path(self_path, "libblazing_recompiled.so");
    assert!(sibling.is_some());
    let path = sibling.unwrap();
    assert_eq!(
        path.to_str().unwrap().replace('\\', "/"),
        "/data/app/~~random/com.blazingrecomp.game/lib/arm64/libblazing_recompiled.so"
    );

    let apk_path = "/data/app/~~abc/base.apk!/lib/arm64-v8a/libblazing_recomp.so";
    let apk_sibling = resolve_sibling_library_path(apk_path, "libblazing_recompiled.so");
    assert!(apk_sibling.is_some());
    assert_eq!(
        apk_sibling.unwrap().to_str().unwrap().replace('\\', "/"),
        "/data/app/~~abc/base.apk!/lib/arm64-v8a/libblazing_recompiled.so"
    );

    let bare_name = "libblazing_recomp.so";
    let bare_res = resolve_sibling_library_path(bare_name, "libblazing_recompiled.so");
    assert!(bare_res.is_some());
    assert_eq!(
        bare_res.unwrap().to_str().unwrap().replace('\\', "/"),
        "libblazing_recompiled.so"
    );
}

/// Verifica que HotspotTracker acumule frecuencias para direcciones ejecutables de GBA
/// (ROM, IWRAM, EWRAM y BIOS), ordene de mayor a menor y limpie el acumulador de forma atómica en cada volcado.
#[test]
fn test_hotspot_tracker_accumulation_and_drain() {
    let _guard = bloquear_acumuladores();
    // Vaciar por completo el acumulador previo
    let _ = drain_top_hotspots(usize::MAX);

    // 1. Direcciones fuera de regiones ejecutables deben ignorarse (ej. registros I/O o SRAM)
    record_hotspot(0x0400_0000);
    record_hotspot(0x0E00_0000);
    assert!(drain_top_hotspots(10).is_empty());

    // 2. Direcciones de ROM, IWRAM y BIOS: acumular frecuencias y limpiar bit Thumb (bit 0)
    record_hotspot(0x0801_2001); // Thumb -> 0x08012000
    record_hotspot(0x0801_2000); // 2 veces
    record_hotspot(0x0801_2001); // 3 veces

    record_hotspot(0x0300_1001); // IWRAM -> 0x03001000 (2 veces)
    record_hotspot(0x0300_1000);

    record_hotspot(0x0000_0020); // BIOS (1 vez)

    let top = drain_top_hotspots(3);
    assert_eq!(top.len(), 3);
    assert_eq!(top[0], (0x0801_2000, 3));
    assert_eq!(top[1], (0x0300_1000, 2));
    assert_eq!(top[2], (0x0000_0020, 1));

    // El acumulador debe haber quedado vacío tras un volcado total
    assert!(drain_top_hotspots(usize::MAX).is_empty());
}

/// Verifica que el volcado consuma únicamente las direcciones que devuelve: la cola no
/// reportada conserva sus conteos y vuelve a competir en el volcado siguiente.
#[test]
fn test_drain_conserva_la_cola_no_reportada() {
    let _guard = bloquear_acumuladores();
    let _ = drain_top_hotspots(usize::MAX);

    // Cuatro direcciones ejecutables con frecuencias distintas (4.000, 3.000, 2.000 y 1.000)
    // para que el orden sea determinista sin depender del desempate de `sort_unstable_by`.
    let caliente_rom = 0x0801_3000u32;
    let caliente_iwram = 0x0300_4000u32;
    let fria_bios = 0x0000_0030u32;
    let fria_rom = 0x0801_4000u32;
    for _ in 0..4_000 {
        record_hotspot(caliente_rom);
    }
    for _ in 0..3_000 {
        record_hotspot(caliente_iwram);
    }
    for _ in 0..2_000 {
        record_hotspot(fria_bios);
    }
    for _ in 0..1_000 {
        record_hotspot(fria_rom);
    }

    // 1. El primer volcado se queda con las dos más frecuentes y las consume.
    assert_eq!(
        drain_top_hotspots(2),
        vec![(caliente_rom, 4_000), (caliente_iwram, 3_000)]
    );

    // 2. Las dos que no entraron en el top sobreviven con sus conteos intactos.
    assert_eq!(
        drain_top_hotspots(2),
        vec![(fria_bios, 2_000), (fria_rom, 1_000)]
    );

    // 3. Caso límite: un límite mayor o igual que el número de direcciones vacía el acumulador.
    let _ = drain_top_hotspots(usize::MAX);
    assert!(drain_top_hotspots(usize::MAX).is_empty());
}

/// Verifica que un límite de cero no consuma ninguna dirección del acumulador.
#[test]
fn test_drain_con_limite_cero_no_consume_nada() {
    let _guard = bloquear_acumuladores();
    let _ = drain_top_hotspots(usize::MAX);

    let dir = 0x0801_5000u32;
    for _ in 0..5_000 {
        record_hotspot(dir);
    }

    assert!(
        drain_top_hotspots(0).is_empty(),
        "un límite de 0 no debe reportar ninguna dirección"
    );
    assert_eq!(
        drain_top_hotspots(1),
        vec![(dir, 5_000)],
        "un límite de 0 no debe consumir contadores"
    );
}

/// Verifica que la ejecución se clasifique por región de memoria y que la línea de
/// telemetría `# regiones` se formatee en el orden canónico rom/iwram/ewram/bios.
#[test]
fn test_contadores_por_region_clasifican_y_formatean() {
    let _guard = bloquear_acumuladores();
    let _ = drain_top_hotspots(usize::MAX);

    // Fronteras de cada región ejecutable y de las que no lo son.
    assert_eq!(region_of(0x0800_0000), Some(Region::Rom));
    assert_eq!(region_of(0x09FF_FFFE), Some(Region::Rom));
    assert_eq!(region_of(0x0A00_0000), None);
    assert_eq!(region_of(0x0300_0000), Some(Region::Iwram));
    assert_eq!(region_of(0x0300_7FFE), Some(Region::Iwram));
    assert_eq!(region_of(0x0300_8000), None);
    assert_eq!(region_of(0x0200_0000), Some(Region::Ewram));
    assert_eq!(region_of(0x0000_3FFE), Some(Region::Bios));
    assert_eq!(region_of(0x4000), None);
    assert_eq!(region_of(0x0400_0000), None);
    assert_eq!(region_of(0x0E00_0000), None);

    // Acumulación: 7 pasos en ROM, 5 en IWRAM, 3 en EWRAM y 2 en BIOS.
    let antes = region_counters();
    for _ in 0..7 {
        record_hotspot(0x0801_6001);
    }
    for _ in 0..5 {
        record_hotspot(0x0300_5000);
    }
    for _ in 0..3 {
        record_hotspot(0x0200_1000);
    }
    for _ in 0..2 {
        record_hotspot(0x0000_0100);
    }
    record_hotspot(0x0400_0000); // Registro de E/S: no es región ejecutable
    record_hotspot(0x0E00_0000); // SRAM: no es región ejecutable
    let despues = region_counters();

    assert_eq!(
        despues.interprete[Region::Rom.index()] - antes.interprete[Region::Rom.index()],
        7
    );
    assert_eq!(
        despues.interprete[Region::Iwram.index()] - antes.interprete[Region::Iwram.index()],
        5
    );
    assert_eq!(
        despues.interprete[Region::Ewram.index()] - antes.interprete[Region::Ewram.index()],
        3
    );
    assert_eq!(
        despues.interprete[Region::Bios.index()] - antes.interprete[Region::Bios.index()],
        2
    );

    // Los bloques nativos se acumulan aparte y con una sola llamada por fotograma.
    let mut nativos = [0u64; 4];
    nativos[Region::Rom.index()] = 11;
    nativos[Region::Iwram.index()] = 4;
    add_native_region_ticks(&nativos);
    let con_nativos = region_counters();
    assert_eq!(
        con_nativos.nativos[Region::Rom.index()] - despues.nativos[Region::Rom.index()],
        11
    );
    assert_eq!(
        con_nativos.nativos[Region::Iwram.index()] - despues.nativos[Region::Iwram.index()],
        4
    );

    // Formateo de la línea de telemetría, con el par `interprete/nativos` por región.
    assert_eq!(Region::NAMES, ["rom", "iwram", "ewram", "bios"]);
    let rc = RegionCounters {
        interprete: [1, 2, 3, 4],
        nativos: [10, 20, 0, 40],
    };
    assert_eq!(rc.to_line(), "# regiones rom=1/10 iwram=2/20 ewram=3/0 bios=4/40");
    assert!(!rc.is_empty());
    assert!(RegionCounters::default().is_empty());
}

/// Verifica el formato de la línea `# ppucamino`, con el reparto de los efectos de color, y el
/// porcentaje de líneas de la vía rápida, incluidos el redondeo y el caso sin líneas contadas.
#[test]
fn test_la_linea_de_camino_lleva_el_desglose() {
    let _guard = bloquear_acumuladores();
    let c = PpuPathCounters {
        rectas: 1280,
        mezcla: 320,
        ventana: 0,
        semi: 0,
        inertes: 640,
        efecto: 320,
        efalpha: 320,
        efbrillo: 0,
        efoscuro: 320,
    };
    assert_eq!(c.total(), 1600, "el total sigue siendo solo las cuatro cuentas");
    assert_eq!(c.rectas_pct(), 80);
    assert_eq!(
        c.to_line(),
        "# ppucamino rectas=1280 mezcla=320 ventana=0 semi=0 inertes=640 efecto=320 efalpha=320 \
         efbrillo=0 efoscuro=320 (rectas=80%)"
    );
    assert!(!c.is_empty());
    // El reparto por modo cubre todas las líneas con efecto real, descartadas o no.
    assert_eq!(c.efalpha + c.efbrillo + c.efoscuro, c.efecto + c.mezcla);

    // Una línea de cada tres recta: 33,3% se imprime como 33.
    let tercios = PpuPathCounters {
        rectas: 1,
        mezcla: 0,
        ventana: 1,
        semi: 1,
        ..PpuPathCounters::default()
    };
    assert_eq!(tercios.rectas_pct(), 33);

    // Frontera tomada de una corrida real en el Android TV: 24,66 % se imprime como 25, que es lo
    // que distingue redondear de truncar (24). Los dos casos de arriba no lo hacen, porque 80 % y
    // 33,3 % imprimen igual por los dos caminos, así que sin esta frontera truncar pasaría.
    let corrida_tv = PpuPathCounters {
        rectas: 14881,
        mezcla: 45440,
        ..PpuPathCounters::default()
    };
    assert_eq!(corrida_tv.total(), 60321, "la suma del volcado es la del log");
    assert_eq!(corrida_tv.rectas_pct(), 25);

    // Sin líneas contadas no se inventa un porcentaje ni se escribe la línea.
    assert!(PpuPathCounters::default().is_empty());
    assert_eq!(PpuPathCounters::default().rectas_pct(), 0);
}
