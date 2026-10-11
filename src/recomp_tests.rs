use super::*;

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
fn test_run_frame_native_preserves_real_bios_stepping() {
    static mut BIOS_CALLED: bool = false;
    extern "C" fn failing_bios(_api: *const gba_core::capi::RtApi, _m: *mut std::ffi::c_void) -> u32 {
        unsafe { BIOS_CALLED = true; }
        0
    }

    let blocks = [RcgBlock { key: 0x0000_0080, func: failing_bios }];
    let table = BlockTable::from_blocks(&blocks);

    let rom = vec![0u8; 1024];
    let bios = vec![0u8; 16384];
    let mut machine = Machine::new_with_bios(rom, &bios);
    machine.cpu.regs[15] = 0x0000_0080;

    let (native, fallback) = run_frame_native(&mut machine, &table, 10);
    assert_eq!(native, 0, "No debe invocar bloques AOT en la BIOS real");
    assert!(fallback > 0, "Debe avanzar en el intérprete paso a paso");
    assert!(!unsafe { BIOS_CALLED }, "El bloque nativo de la BIOS no debe ejecutarse");
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

/// Verifica que HotspotTracker acumule frecuencias únicamente para direcciones de la ROM de GBA,
/// ordene de mayor a menor y limpie el acumulador de forma atómica en cada volcado.
#[test]
fn test_hotspot_tracker_accumulation_and_drain() {
    // Vaciar acumulador previo
    let _ = drain_top_hotspots(100);

    // 1. Direcciones fuera de la ROM deben ignorarse (BIOS e IWRAM)
    record_hotspot(0x0000_0020);
    record_hotspot(0x0300_1000);
    assert!(drain_top_hotspots(10).is_empty());

    // 2. Direcciones de ROM: acumular frecuencias y limpiar bit Thumb (bit 0)
    record_hotspot(0x0801_2001); // Thumb -> 0x08012000
    record_hotspot(0x0801_2000); // 2 veces
    record_hotspot(0x0801_2001); // 3 veces

    record_hotspot(0x0802_4001); // 1 vez
    record_hotspot(0x0802_4000); // 2 veces

    record_hotspot(0x0803_8001); // 1 vez

    let top = drain_top_hotspots(2);
    assert_eq!(top.len(), 2);
    assert_eq!(top[0], (0x0801_2000, 3));
    assert_eq!(top[1], (0x0802_4000, 2));

    // El acumulador debe haber quedado vacío tras el drain
    assert!(drain_top_hotspots(10).is_empty());
}
