//! Tests unitarios del motor de audio nativo.

use super::*;
use std::fs;

#[test]
fn test_audio_resampler_and_ring_buffer_transfer() {
    // Escenario 1: Resampling lineal exacto y transferencia al búfer circular
    // 1092 frames estéreo a 65536 Hz (2184 muestras i16 entrelazadas)
    let in_frames = 1092;
    let mut input = Vec::with_capacity(in_frames * 2);
    for i in 0..in_frames {
        let sample_l = ((i % 100) as i16) * 100;
        let sample_r = -sample_l;
        input.push(sample_l);
        input.push(sample_r);
    }

    let mut resampler = AudioResampler::new(AUDIO_RATE_GBA, DEFAULT_SAMPLE_RATE);
    let mut resampled = Vec::new();
    resampler.resample(&input, &mut resampled);

    let out_frames = resampled.len() / 2;
    // 1092 * 48000 / 65536 = 799.8 -> 800 frames (+- 1)
    assert!(
        out_frames >= 799 && out_frames <= 801,
        "El resampler debe producir ~800 frames a 48kHz (obtenido: {out_frames})"
    );

    let ring = AudioRingBuffer::new(2048);
    let pushed = ring.push(&resampled);
    assert_eq!(
        pushed, out_frames,
        "Debe insertar todos los frames remuestreados"
    );

    let mut consumer_buf = vec![0i16; out_frames * 2];
    let popped = ring.pop(&mut consumer_buf);
    assert_eq!(
        popped, out_frames,
        "Debe retirar todos los frames insertados"
    );
    assert_eq!(
        consumer_buf[0], resampled[0],
        "La primera muestra L debe coincidir"
    );
    assert_eq!(
        consumer_buf[1], resampled[1],
        "La primera muestra R debe coincidir"
    );
}

#[test]
fn test_audio_ring_buffer_underrun_graceful_fill() {
    // Escenario 2: Resistencia a subdesbordamiento del búfer circular sin caídas
    let ring = AudioRingBuffer::new(1024);

    // Insertar solo 64 frames (128 muestras)
    let short_input = vec![1234i16; 64 * 2];
    ring.push(&short_input);

    // El consumidor solicita 256 frames (512 muestras)
    let mut dest = vec![-1i16; 256 * 2];
    let valid_frames = ring.pop(&mut dest);

    assert_eq!(
        valid_frames, 64,
        "Debe reportar solo 64 frames válidos leídos"
    );
    assert_eq!(
        ring.underruns(),
        1,
        "Debe contabilizar 1 evento de underrun"
    );

    // Los primeros 64 frames deben tener los datos válidos
    for i in 0..128 {
        assert_eq!(
            dest[i], 1234,
            "Muestras leídas deben coincidir con la entrada"
        );
    }

    // Los siguientes 192 frames (384 muestras) deben rellenarse con silencio (0)
    for i in 128..512 {
        assert_eq!(dest[i], 0, "Muestras de underrun deben rellenarse con 0");
    }
}

#[test]
fn test_audio_buf_drained_each_frame_bounds_memory() {
    // Escenario 3: Drenaje de machine.bus.audio_buf evita acumulación
    // Crea una ROM mínima sintética
    let mut rom = vec![0u8; 1024 * 1024];
    rom[0] = 0x2E;
    rom[3] = 0xEA;
    let mut runner = crate::runner::GameRunner::new(rom, None, screen::ScreenKind::Frontlit);

    for frame in 1..=10 {
        runner.step_frame();
        let drained = runner.drain_audio_samples();
        assert!(
            !drained.is_empty(),
            "Fotograma {frame} debe haber generado muestras de audio"
        );
        assert!(
            runner.machine.bus.audio_buf.len() < 2300,
            "machine.bus.audio_buf debe quedar vaciado o bajo el presupuesto de 1 fotograma (actual: {})",
            runner.machine.bus.audio_buf.len()
        );
    }
}

#[test]
fn test_audio_flag_off_disables_stream() {
    // Escenario 4: Detección de bandera de apagado de audio en caliente
    let temp_dir = std::env::temp_dir().join(format!("blazing_audio_test_{}", std::process::id()));
    let _ = fs::create_dir_all(&temp_dir);
    let flag_file = temp_dir.join(NOMBRE_BANDERA_AUDIO_APAGADO);

    // Sin bandera -> no debe estar apagada
    assert!(
        !audio_apagado_por_marca(&[temp_dir.clone()]),
        "Sin el archivo de bandera, el audio debe permanecer activo"
    );

    // Crear la bandera
    fs::write(&flag_file, b"").expect("escribir flag");
    assert!(
        audio_apagado_por_marca(&[temp_dir.clone()]),
        "Con el archivo de bandera presente, el audio debe reportarse apagado"
    );

    let _ = fs::remove_file(&flag_file);
    let _ = fs::remove_dir_all(&temp_dir);
}

/// Construye una imagen ROM sintética que incluye la firma MP2K oficial.
fn create_rom_with_mp2k() -> Vec<u8> {
    let mut rom = vec![0u8; 1024 * 1024];
    rom[0] = 0x2E;
    rom[3] = 0xEA;

    // Incrustar la estructura del literal-pool de SoundMain en 0x1000
    let off = 0x1000;
    rom[off..off + 4].copy_from_slice(&0x0300_7FF0u32.to_le_bytes()); // needle_lo
    rom[off + 4..off + 8].copy_from_slice(&0x6873_6D53u32.to_le_bytes()); // needle_hi 'Smsh'
    rom[off + 8..off + 12].copy_from_slice(&0x0300_2451u32.to_le_bytes()); // sound_main_ram en IWRAM
    rom[off + 12..off + 16].copy_from_slice(&0x0400_0006u32.to_le_bytes()); // REG_VCOUNT
    rom[off + 16..off + 20].copy_from_slice(&0x0000_0350u32.to_le_bytes()); // o_pcmBuffer
    rom
}

#[test]
fn test_mp2k_armed_on_supported_rom() {
    let rom = create_rom_with_mp2k();
    let runner = crate::runner::GameRunner::new(rom, None, screen::ScreenKind::Frontlit);

    assert!(
        runner.machine.bus.mp2k.is_some(),
        "El shadow mixer MP2K debe armarse automáticamente en una ROM con firma MP2K"
    );
    assert!(
        runner.machine.bus.tap_channels,
        "tap_channels debe estar activo para que el generador recolecte hle_tap y psg_tap"
    );
}

#[test]
fn test_enhanced_audio_mix_combines_hle_and_psg() {
    let hle = vec![0.25f32, -0.25f32, 0.5f32, -0.5f32]; // 2 frames estéreo
    let psg = vec![1000i16, -1000i16, 2000i16, -2000i16]; // 2 frames estéreo

    let mut dest = Vec::new();
    mix_enhanced_samples(&hle, &psg, &mut dest);

    assert_eq!(dest.len(), 4, "Debe generar 4 muestras (2 pares estéreo)");
    // 0.25 * 32768.0 = 8192 -> 8192 + 1000 = 9192
    assert_eq!(dest[0], 9192);
    // -0.25 * 32768.0 = -8192 -> -8192 - 1000 = -9192
    assert_eq!(dest[1], -9192);
    // 0.5 * 32768.0 = 16384 -> 16384 + 2000 = 18384
    assert_eq!(dest[2], 18384);
    assert_eq!(dest[3], -18384);
}

#[test]
fn test_enhanced_audio_falls_back_when_not_live() {
    let rom = create_rom_with_mp2k();
    let mut runner = crate::runner::GameRunner::new(rom, None, screen::ScreenKind::Frontlit);

    // En el fotograma 1, MP2K no ha sido ejecutado por la CPU, por lo que !live()
    runner.step_frame();

    let samples = runner.drain_audio_samples();
    assert!(
        !samples.is_empty(),
        "Debe entregar muestras del camino fiel si MP2K aún no está probado"
    );
}

#[test]
fn test_enhanced_audio_flag_off_reverts_to_faithful() {
    let temp_dir =
        std::env::temp_dir().join(format!("blazing_audio_enh_test_{}", std::process::id()));
    let _ = fs::create_dir_all(&temp_dir);
    let flag_file = temp_dir.join(NOMBRE_BANDERA_AUDIO_MEJORADO_APAGADO);

    // Sin bandera -> el audio mejorado debe estar ACTIVO por defecto
    assert!(
        audio_mejorado_activado(&[temp_dir.clone()]),
        "El audio mejorado debe estar activo por defecto si no existe la bandera de apagado"
    );

    // Con bandera de apagado presente -> debe desactivarse
    fs::write(&flag_file, b"").expect("escribir flag");
    assert!(
        !audio_mejorado_activado(&[temp_dir.clone()]),
        "La bandera audio-enhanced-off.flag debe desactivar el audio mejorado"
    );

    let _ = fs::remove_file(&flag_file);
    let _ = fs::remove_dir_all(&temp_dir);
}

#[test]
fn test_audio_ring_buffer_initial_priming() {
    let mut engine = NativeAudioEngine::new(DEFAULT_SAMPLE_RATE);
    assert_eq!(engine.ring_buffer().available_frames(), 0);
    assert_eq!(engine.ring_buffer().underruns(), 0);

    engine.start().expect("iniciar engine");
    assert!(
        engine.ring_buffer().available_frames() >= TARGET_WATERMARK_FRAMES,
        "El búfer debe contener al menos TARGET_WATERMARK_FRAMES tras el pre-cebado (obtenido: {})",
        engine.ring_buffer().available_frames()
    );
    assert_eq!(
        engine.ring_buffer().underruns(),
        0,
        "No debe haber underruns tras el cebado"
    );

    // Consumir un bloque de hardware y verificar que se extrae silencio sin causar underrun
    let mut dest = vec![1i16; 512 * 2];
    let popped = engine.ring_buffer().pop(&mut dest);
    assert_eq!(popped, 512);
    assert_eq!(dest[0], 0, "Las muestras de cebado deben ser silencio (0)");
    assert_eq!(dest[1], 0);
    assert_eq!(engine.ring_buffer().underruns(), 0);
}

#[test]
fn test_audio_ring_buffer_zero_underruns_at_steady_rate() {
    let ring = AudioRingBuffer::new(DEFAULT_RING_CAPACITY_FRAMES);
    // Cebar inicialmente para simular el arranque del motor de audio
    ring.prime(TARGET_WATERMARK_FRAMES);

    // Simular 60 ciclos continuos a 60 FPS (1 segundo completo de reproducción)
    // Productor: 800 frames por ciclo (48.000 frames/segundo)
    // Consumidor: 800 frames por ciclo (48.000 frames/segundo)
    let producer_block = vec![1000i16; 800 * 2];
    let mut consumer_block = vec![0i16; 800 * 2];

    for _ in 0..60 {
        let pushed = ring.push(&producer_block);
        assert_eq!(pushed, 800);

        let popped = ring.pop(&mut consumer_block);
        assert_eq!(popped, 800);
    }

    assert_eq!(
        ring.underruns(),
        0,
        "No debe haber underruns en régimen estacionario sincronizado a 60 FPS"
    );
    assert!(
        ring.available_frames() >= TARGET_WATERMARK_FRAMES,
        "El nivel de marca de agua debe mantenerse estable"
    );
}
