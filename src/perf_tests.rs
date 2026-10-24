use super::*;

#[test]
fn test_informe_promedia_y_reporta_peor_caso() {
    let mut perf = FramePerf::new();
    let timings = [
        FrameTiming {
            ejec_ms: 20.0,
            color_ms: 1.0,
            dibujo_ms: 10.0,
            espera_ms: 2.0,
            subida_ms: 1.0,
            codificacion_ms: 2.0,
            present_ms: 7.0,
            nativos: 1000,
            interprete: 10,
            ppu_ms: 3.0,
            ppu_bg_ms: 1.5,
            ppu_sprite_ms: 0.5,
            ppu_compose_ms: 1.0,
            apu_ms: 1.0, pump_llamadas: 0, pump_muestras: 0,
            lecturas: 50000,
            escrituras: 10000,
        },
        FrameTiming {
            ejec_ms: 20.0,
            color_ms: 1.0,
            dibujo_ms: 10.0,
            espera_ms: 2.0,
            subida_ms: 1.0,
            codificacion_ms: 2.0,
            present_ms: 7.0,
            nativos: 1000,
            interprete: 10,
            ppu_ms: 3.0,
            ppu_bg_ms: 1.5,
            ppu_sprite_ms: 0.5,
            ppu_compose_ms: 1.0,
            apu_ms: 1.0, pump_llamadas: 0, pump_muestras: 0,
            lecturas: 50000,
            escrituras: 10000,
        },
        FrameTiming {
            ejec_ms: 20.0,
            color_ms: 1.0,
            dibujo_ms: 10.0,
            espera_ms: 2.0,
            subida_ms: 1.0,
            codificacion_ms: 2.0,
            present_ms: 7.0,
            nativos: 1000,
            interprete: 10,
            ppu_ms: 3.0,
            ppu_bg_ms: 1.5,
            ppu_sprite_ms: 0.5,
            ppu_compose_ms: 1.0,
            apu_ms: 1.0, pump_llamadas: 0, pump_muestras: 0,
            lecturas: 50000,
            escrituras: 10000,
        },
        FrameTiming {
            ejec_ms: 20.0,
            color_ms: 1.0,
            dibujo_ms: 30.0,
            espera_ms: 10.0,
            subida_ms: 1.0,
            codificacion_ms: 2.0,
            present_ms: 27.0,
            nativos: 1000,
            interprete: 10,
            ppu_ms: 3.0,
            ppu_bg_ms: 1.5,
            ppu_sprite_ms: 0.5,
            ppu_compose_ms: 1.0,
            apu_ms: 1.0, pump_llamadas: 0, pump_muestras: 0,
            lecturas: 50000,
            escrituras: 10000,
        },
    ];

    for t in timings {
        perf.record_frame(t);
    }
    for _ in 0..7 {
        perf.record_empty_turn();
    }

    let report = perf.close_window((1200, 1920));
    assert_eq!(report.present_max_ms, 27.0);
    assert_eq!(
        report.to_line(),
        "t=1s fps=4.0 ejec=20.0/20.0 ppu=3.0 ppubg=1.5 ppuspr=0.5 ppucomp=1.0 apu=1.0 lecturas=50000 escrituras=10000 color=1.0/1.0 dibujo=15.0/30.0 espera=4.0/10.0 subida=1.0 codificacion=2.0 present=12.0 presentmax=27.0 nativos=1000 interprete=10 vacias=7 ventana=1200x1920 pump=0 pumpm=0 formato=2"
    );
}

#[test]
fn test_report_breaks_down_draw_time() {
    let mut perf = FramePerf::new();
    perf.record_frame(FrameTiming {
        ejec_ms: 12.0,
        color_ms: 1.0,
        dibujo_ms: 55.0,
        espera_ms: 0.6,
        subida_ms: 3.0,
        codificacion_ms: 4.0,
        present_ms: 48.0,
        nativos: 10,
        interprete: 20,
        ppu_ms: 2.0,
        ppu_bg_ms: 1.0,
        ppu_sprite_ms: 0.5,
        ppu_compose_ms: 0.5,
        apu_ms: 0.5, pump_llamadas: 0, pump_muestras: 0,
        lecturas: 1000,
        escrituras: 500,
    });

    let report = perf.close_window((1200, 2000));
    let line = report.to_line();

    assert!(line.contains("subida=3.0"), "la línea debe reportar la subida: {line}");
    assert!(line.contains("codificacion=4.0"), "la línea debe reportar la codificación: {line}");
    assert!(line.contains("present=48.0"), "la línea debe reportar la presentación: {line}");
    assert!(line.contains("ppu=2.0"), "la línea debe reportar la PPU: {line}");
    assert!(line.contains("ppubg=1.0"), "la línea debe reportar fondos PPU: {line}");
    assert!(line.contains("ppuspr=0.5"), "la línea debe reportar sprites PPU: {line}");
    assert!(line.contains("ppucomp=0.5"), "la línea debe reportar composición PPU: {line}");
    assert!(line.contains("apu=0.5"), "la línea debe reportar el APU: {line}");
    assert!(line.contains("lecturas=1000"), "la línea debe reportar las lecturas: {line}");
    assert!(line.contains("escrituras=500"), "la línea debe reportar las escrituras: {line}");
    assert!(
        report.dibujo_avg_ms >= report.present_avg_ms,
        "el total de dibujo debe cubrir la fase de presentación"
    );
    assert_eq!(
        report.subida_avg_ms + report.codificacion_avg_ms + report.present_avg_ms,
        report.dibujo_avg_ms,
        "las tres fases deben sumar el total de dibujo"
    );
}

#[test]
fn test_ventana_sin_fotogramas_no_divide_por_cero() {
    let mut perf = FramePerf::new();
    for _ in 0..1000 {
        perf.record_empty_turn();
    }

    let report = perf.close_window((1200, 1920));
    assert_eq!(report.present_max_ms, 0.0);
    assert_eq!(
        report.to_line(),
        "t=1s fps=0.0 ejec=0.0/0.0 ppu=0.0 ppubg=0.0 ppuspr=0.0 ppucomp=0.0 apu=0.0 lecturas=0 escrituras=0 color=0.0/0.0 dibujo=0.0/0.0 espera=0.0/0.0 subida=0.0 codificacion=0.0 present=0.0 presentmax=0.0 nativos=0 interprete=0 vacias=1000 ventana=1200x1920 pump=0 pumpm=0 formato=2"
    );
}

#[test]
fn test_medicion_costo_reloj_retorna_valor_positivo() {
    let cost = measure_clock_cost_ns();
    assert!(cost > 0, "el costo de lectura de reloj debe ser >= 1 ns");
}

#[test]
fn test_resolve_perf_candidates_prioritizes_downloads() {
    let extra = vec![
        std::path::PathBuf::from("/storage/emulated/0/Android/data/com.blazingrecomp.game/files"),
        std::path::PathBuf::from("/sdcard/Download"),
    ];
    let candidates = resolve_perf_candidates(&extra);
    assert_eq!(candidates[0], std::path::PathBuf::from("/sdcard/Download"));
    assert_eq!(candidates[1], std::path::PathBuf::from("/storage/emulated/0/Download"));
    assert_eq!(
        candidates[2],
        std::path::PathBuf::from("/storage/emulated/0/Android/data/com.blazingrecomp.game/files")
    );
    // Verificar que no hay duplicados de /sdcard/Download
    assert_eq!(candidates.iter().filter(|p| **p == std::path::PathBuf::from("/sdcard/Download")).count(), 1);
}

#[test]
fn test_informe_reporta_peor_caso_de_presentacion() {
    let mut perf = FramePerf::new();
    let frame_base = FrameTiming {
        ejec_ms: 10.0,
        color_ms: 1.0,
        dibujo_ms: 5.0,
        espera_ms: 0.1,
        subida_ms: 1.0,
        codificacion_ms: 1.0,
        present_ms: 2.0,
        nativos: 100,
        interprete: 0,
        ppu_ms: 2.0,
        ppu_bg_ms: 1.0,
        ppu_sprite_ms: 0.5,
        ppu_compose_ms: 0.5,
        apu_ms: 0.5, pump_llamadas: 0, pump_muestras: 0,
        lecturas: 1000,
        escrituras: 500,
    };

    // Frame 1: presentación normal rápida (2.0 ms)
    perf.record_frame(frame_base.clone());

    // Frame 2: stall severo en present() (500.0 ms)
    let mut frame_stall = frame_base;
    frame_stall.present_ms = 500.0;
    frame_stall.dibujo_ms = 503.0;
    perf.record_frame(frame_stall);

    let report = perf.close_window((1280, 720));

    // El promedio no se altera: (2.0 + 500.0) / 2 = 251.0 ms
    assert_eq!(report.present_avg_ms, 251.0);
    // El peor caso captura el bloqueo aislado exacto:
    assert_eq!(report.present_max_ms, 500.0);

    let line = report.to_line();
    assert!(line.contains("present=251.0 presentmax=500.0"));
}

#[test]
fn test_report_desglose_ppu_subfases() {
    let mut perf = FramePerf::new();
    perf.record_frame(FrameTiming {
        ejec_ms: 10.0,
        color_ms: 0.5,
        dibujo_ms: 2.0,
        espera_ms: 0.0,
        subida_ms: 0.5,
        codificacion_ms: 0.5,
        present_ms: 1.0,
        nativos: 100,
        interprete: 0,
        ppu_ms: 5.5,
        ppu_bg_ms: 2.5,
        ppu_sprite_ms: 1.0,
        ppu_compose_ms: 2.0,
        apu_ms: 0.8, pump_llamadas: 0, pump_muestras: 0,
        lecturas: 1000,
        escrituras: 500,
    });

    let report = perf.close_window((1280, 720));
    assert_eq!(report.ppu_avg_ms, 5.5);
    assert_eq!(report.ppu_bg_avg_ms, 2.5);
    assert_eq!(report.ppu_sprite_avg_ms, 1.0);
    assert_eq!(report.ppu_compose_avg_ms, 2.0);

    let line = report.to_line();
    assert!(line.contains("ppu=5.5 ppubg=2.5 ppuspr=1.0 ppucomp=2.0"));
}

#[test]
fn test_ppu_hw_muestreo_y_linea_telemetria() {
    let mut perf = FramePerf::new();
    
    // Fotograma 1: GPU, Modo 0, con ventana activa, sin blend, 20 sprites (5 de 8bpp, 2 afines)
    perf.record_ppu_hw(PpuHwFrameSample {
        is_gpu: true,
        ppu_nativa: false,
        is_mode0: true,
        window_active: true,
        blend_active: false,
        spr_total: 20,
        spr_8bpp: 5,
        spr_affine: 2,
    });
    perf.record_frame(FrameTiming::default());

    // Fotograma 2: Software fallback, Modo 0, sin ventana, con blend, 30 sprites (15 de 8bpp, 0 afines)
    perf.record_ppu_hw(PpuHwFrameSample {
        is_gpu: false,
        ppu_nativa: false,
        is_mode0: true,
        window_active: false,
        blend_active: true,
        spr_total: 30,
        spr_8bpp: 15,
        spr_affine: 0,
    });
    perf.record_frame(FrameTiming::default());

    let report = perf.close_window((1280, 720));
    assert_eq!(report.ppu_hw.gpu_frames, 1);
    assert_eq!(report.ppu_hw.sw_frames, 1);
    assert_eq!(report.ppu_hw.mode0_frames, 2);
    assert_eq!(report.ppu_hw.window_frames, 1);
    assert_eq!(report.ppu_hw.blend_frames, 1);
    assert_eq!(report.ppu_hw.spr_total_avg, 25);
    assert_eq!(report.ppu_hw.spr_8bpp_avg, 10);
    assert_eq!(report.ppu_hw.spr_affine_avg, 1);

    let line = report.ppu_hw.to_line();
    assert_eq!(
        line,
        "# ppu_hw gpu=1 sw=1 modo0=2 win=1 blend=1 spr_total=25 spr_8bpp=10 spr_afin=1 nativa=0"
    );
}

#[test]
fn test_ppu_hw_distingue_los_fotogramas_de_la_ppu_nativa() {
    let mut perf = FramePerf::new();

    // 30 fotogramas dibujados por la PPU nativa...
    for _ in 0..30 {
        perf.record_ppu_hw(PpuHwFrameSample {
            is_gpu: true,
            ppu_nativa: true,
            ..Default::default()
        });
        perf.record_frame(FrameTiming::default());
    }

    // ...y 30 por el shim, que también va por GPU pero no es la PPU nativa.
    for _ in 0..30 {
        perf.record_ppu_hw(PpuHwFrameSample {
            is_gpu: true,
            ppu_nativa: false,
            ..Default::default()
        });
        perf.record_frame(FrameTiming::default());
    }

    let report = perf.close_window((1280, 720));
    assert_eq!(report.ppu_hw.gpu_frames, 60);
    assert_eq!(report.ppu_hw.nativa_frames, 30);

    let line = report.ppu_hw.to_line();
    assert!(
        line.contains("gpu=60"),
        "la línea debe contar los fotogramas por GPU: {line}"
    );
    assert!(
        line.contains("nativa=30"),
        "la línea debe distinguir los fotogramas de la PPU nativa: {line}"
    );
}
