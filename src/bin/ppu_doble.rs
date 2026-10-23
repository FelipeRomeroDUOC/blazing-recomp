//! Visor interactivo de doble pantalla para comparar en vivo la PPU por software
//! y la PPU nativa (GPU) de Fire Emblem: The Blazing Blade.
//!
//! Abre una ventana de 1440x480 (escala 3x) dividida lado a lado:
//! - Mitad izquierda (240x160): PPU de referencia por software (L alterna entre fin de cuadro y por línea).
//! - Mitad derecha (240x160): PPU nativa acelerada por GPU.
//!
//! Controles GBA:
//! - Flechas: D-Pad
//! - Z: Botón A
//! - X: Botón B
//! - Enter: Start
//! - Backspace: Select
//! - A: Gatillo L
//! - S: Gatillo R
//!
//! Controles del arnés:
//! - F5: Forzar captura PNG compuesta (960x160) y diagnóstico en `volcados/diferencias/`
//! - Tab: Alternar resaltado en magenta de píxeles distintos sobre el panel derecho
//! - L: Alternar panel izquierdo entre referencia por línea (emulador) y estado fin de cuadro
//! - Espacio: Pausar / reanudar emulación
//! - N: Avanzar 1 fotograma cuando está en pausa
//! - Shift: Modo turbo (sin limitador de 60 FPS)
//! - Esc: Salir
//!
//! Uso:
//!   cargo run --release --features proto --bin ppu_doble -- <ruta.gba> [ruta_bios.bin]

#[path = "../../examples/comun/mod.rs"]
mod comun;

use std::env;
use std::fs;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::sync_channel;
use std::sync::Arc;
use std::thread;
use std::time::{Duration, Instant};

use comun::color::software_a_rgb;
use comun::diagnostico::{
    self, evaluar_cuadro_en_vivo, guardar_captura_forzada, guardar_captura_si_difiere,
    lienzo_doble_bgrx, render_software_fin, EstadoRef, FiltroFirmas, ALTO, ANCHO_DOBLE,
};
use comun::gpu::Gpu;
use comun::registro::io16;
use gba_core::mem::CapturaLinea;

#[cfg(not(windows))]
fn main() {
    eprintln!("Error: 'ppu_doble' requiere Windows para la presentación interactiva.");
    std::process::exit(1);
}

#[cfg(windows)]
fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        eprintln!("Uso: cargo run --release --features proto --bin ppu_doble -- <ruta.gba> [ruta_bios.bin]");
        std::process::exit(1);
    }

    let ruta_rom = PathBuf::from(&args[1]);
    let rom = fs::read(&ruta_rom).unwrap_or_else(|e| {
        eprintln!("Error al leer la ROM en {}: {e}", ruta_rom.display());
        std::process::exit(1);
    });

    let bios = if args.len() >= 3 {
        let ruta_bios = PathBuf::from(&args[2]);
        fs::read(&ruta_bios).ok()
    } else {
        fs::read("roms/gba_bios.bin").ok()
    };

    println!("============================================================");
    println!(" VISOR DE DOBLE PANTALLA PPU (SOFTWARE VS. NATIVA)");
    println!("============================================================");
    println!("ROM: {} ({} bytes)", ruta_rom.display(), rom.len());
    if let Some(b) = &bios {
        println!("BIOS: cargada ({} bytes)", b.len());
    } else {
        println!("BIOS: no encontrada (usando HLE integrado)");
    }
    println!("------------------------------------------------------------");
    println!("Controles GBA: Flechas (D-Pad), Z (A), X (B), Enter (Start),");
    println!("               Backspace (Select), A (L), S (R)");
    println!("Herramientas:  F5 (Captura forzada), Tab (Resaltar diferencias),");
    println!("               L (Alternar ref fin/línea), Espacio (Pausar),");
    println!("               N (Paso a paso), Shift (Turbo)");
    println!("------------------------------------------------------------");

    ejecutar_visor(rom, bios);
}

/// Trabajo enviado al hilo en segundo plano para no demorar el bucle a 60 FPS.
struct TareaDiagnostico {
    cuadro: u32,
    motivo: String, // "fin", "linea" o "manual"
    sw_linea_rgb: Vec<u8>,
    nat_rgb: Vec<u8>,
    vram: Vec<u8>,
    oam: Vec<u8>,
    palette: Vec<u8>,
    io: Vec<u8>,
    capturas_linea: Vec<CapturaLinea>,
}

#[cfg(windows)]
fn ejecutar_visor(rom: Vec<u8>, bios: Option<Vec<u8>>) {
    use windows_sys::Win32::Foundation::*;
    use windows_sys::Win32::Graphics::Gdi::*;
    use windows_sys::Win32::System::LibraryLoader::GetModuleHandleW;
    use windows_sys::Win32::UI::Input::KeyboardAndMouse::*;
    use windows_sys::Win32::UI::WindowsAndMessaging::*;

    let dir_diferencias = PathBuf::from("volcados").join("diferencias");
    let (tx_diag, rx_diag) = sync_channel::<TareaDiagnostico>(16);
    let en_ejecucion = Arc::new(AtomicBool::new(true));

    // Hilo trabajador para diagnóstico causal y escritura en disco
    let en_ejecucion_hilo = Arc::clone(&en_ejecucion);
    let rom_para_hilo = rom.clone();
    let hilo_diag = thread::spawn(move || {
        let mut gpu_ablacion = match Gpu::nueva_nativa() {
            Ok(g) => Some(g),
            Err(e) => {
                eprintln!("[Diagnóstico] Aviso: GPU secundaria no disponible ({e}), ablación desactivada.");
                None
            }
        };

        while en_ejecucion_hilo.load(Ordering::Relaxed) {
            let Ok(tarea) = rx_diag.recv_timeout(Duration::from_millis(100)) else {
                continue;
            };

            // Reconstruir sw_fin real con el estado de fin de cuadro
            let sw_fin_rgb = render_software_fin(
                &rom_para_hilo,
                &tarea.vram,
                &tarea.oam,
                &tarea.palette,
                &tarea.io,
            );

            if tarea.motivo == "manual" {
                match guardar_captura_forzada(&dir_diferencias, tarea.cuadro, "manual", &sw_fin_rgb, &tarea.nat_rgb) {
                    Ok(ruta) => println!("[F5] Captura manual guardada: {}", ruta.display()),
                    Err(e) => eprintln!("[F5] Error al guardar captura: {e}"),
                }
                continue;
            }

            let fin_estado = EstadoRef {
                io: &tarea.io,
                palette: &tarea.palette,
                vram: &tarea.vram,
                oam: &tarea.oam,
            };
            let lineas_estado: Vec<EstadoRef> = tarea
                .capturas_linea
                .iter()
                .map(|c| EstadoRef {
                    io: &c.io,
                    palette: &c.palette,
                    vram: &c.vram,
                    oam: &c.oam,
                })
                .collect();

            // Diagnóstico causal riguroso
            let mut gpu_ref = gpu_ablacion.as_mut();
            let rom_ablacion = rom_para_hilo.clone();
            let vram_clone = tarea.vram.clone();
            let oam_clone = tarea.oam.clone();
            let pal_clone = tarea.palette.clone();

            let diag = diagnostico::diagnosticar(
                &tarea.nat_rgb,
                &sw_fin_rgb,
                &tarea.sw_linea_rgb,
                fin_estado,
                &lineas_estado,
                |_, io_v| {
                    if let Some(ref mut gpu) = gpu_ref {
                        let nat_v = gpu.renderiza(&vram_clone, &oam_clone, &pal_clone, io_v);
                        let sw_v = render_software_fin(&rom_ablacion, &vram_clone, &oam_clone, &pal_clone, io_v);
                        diagnostico::diferencias(&sw_v, &nat_v).map_or(0, |d| d.distintos)
                    } else {
                        0
                    }
                },
            );

            // Guardar capturas de los casos que difieren
            if diag.dif_fin > 0 && tarea.motivo == "fin" {
                let _ = guardar_captura_si_difiere(&dir_diferencias, tarea.cuadro, "fin", &sw_fin_rgb, &tarea.nat_rgb);
            }
            if diag.dif_linea > 0 {
                let _ = guardar_captura_si_difiere(&dir_diferencias, tarea.cuadro, "linea", &tarea.sw_linea_rgb, &tarea.nat_rgb);
            }

            // Escribir informe diagnóstico completo
            let informe_lineas = diag.informe();
            let txt_ruta = dir_diferencias.join(format!("cuadro-{}.txt", tarea.cuadro));
            let mut txt_contenido = format!("cuadro: {}\nmotivo_disparo: {}\n", tarea.cuadro, tarea.motivo);
            txt_contenido.push_str(&informe_lineas.join("\n"));
            txt_contenido.push('\n');
            let _ = fs::write(&txt_ruta, txt_contenido);

            println!(
                "\n[Cuadro {}] Diagnóstico generado (dif_fin: {}, dif_linea: {}): causa={}",
                tarea.cuadro, diag.dif_fin, diag.dif_linea, diag.causa
            );
            if !diag.registros.is_empty() {
                println!("  -> Registros que cambiaron entre líneas: {:?}", diag.registros);
            }
            if !diag.resuelve.is_empty() {
                println!("  -> Variantes que resuelven: {:?}", diag.resuelve);
            }
            if let Some(t) = &diag.testigo {
                println!("  -> Píxel testigo en ({}, {}): ventana={}, sprites={:?}", t.x, t.y, t.ventana, t.sprites);
            }
        }
    });

    // Inicializar máquina GBA principal
    let mut maquina = gba_core::Machine::new(rom);
    if let Some(b) = bios {
        maquina.bus.bios = b;
    }

    // Inicializar GPU principal
    let mut gpu = match Gpu::nueva_nativa() {
        Ok(g) => g,
        Err(e) => {
            eprintln!("Error al inicializar la GPU para la PPU nativa: {e}");
            en_ejecucion.store(false, Ordering::Relaxed);
            let _ = hilo_diag.join();
            return;
        }
    };

    // Configurar ventana Win32
    unsafe {
        let clase_nombre = encode_wide("PpuDobleClase\0");
        let titulo = encode_wide("Blazing Recomp - Visor Doble PPU (Software vs Nativa)\0");
        let h_instance = GetModuleHandleW(std::ptr::null());

        let wc = WNDCLASSW {
            style: CS_HREDRAW | CS_VREDRAW,
            lpfnWndProc: Some(DefWindowProcW),
            cbClsExtra: 0,
            cbWndExtra: 0,
            hInstance: h_instance,
            hIcon: std::ptr::null_mut(),
            hCursor: LoadCursorW(std::ptr::null_mut(), IDC_ARROW),
            hbrBackground: (COLOR_WINDOW + 1) as HBRUSH,
            lpszMenuName: std::ptr::null(),
            lpszClassName: clase_nombre.as_ptr(),
        };
        RegisterClassW(&wc);

        let ancho_inicial = (ANCHO_DOBLE * 3) as i32; // 1440
        let alto_inicial = (ALTO * 3) as i32;        // 480

        let mut rect = RECT {
            left: 0,
            top: 0,
            right: ancho_inicial,
            bottom: alto_inicial,
        };
        let estilo = WS_OVERLAPPEDWINDOW | WS_VISIBLE;
        AdjustWindowRect(&mut rect, estilo, 0);

        let hwnd = CreateWindowExW(
            0,
            clase_nombre.as_ptr(),
            titulo.as_ptr(),
            estilo,
            CW_USEDEFAULT,
            CW_USEDEFAULT,
            rect.right - rect.left,
            rect.bottom - rect.top,
            std::ptr::null_mut(),
            std::ptr::null_mut(),
            h_instance,
            std::ptr::null(),
        );

        if hwnd == std::ptr::null_mut() {
            eprintln!("Error al crear la ventana de Windows.");
            en_ejecucion.store(false, Ordering::Relaxed);
            let _ = hilo_diag.join();
            return;
        }

        let hdc = GetDC(hwnd);

        // Bucle de juego y presentación
        let mut filtro = FiltroFirmas::default();
        let mut cuadro: u32 = 0;
        let mut pausado = false;
        let mut paso_unico = false;
        let mut resaltar_dif = false;
        let mut ver_ref_fin = false; // Tecla L: false = línea (emulador), true = fin de cuadro
        let mut capturas_totales = 0usize;

        let mut ultimo_fps = Instant::now();
        let mut cuadros_en_segundo = 0u32;
        #[allow(unused_assignments)]
        let mut fps_actual = 60.0f32;

        let frame_duration = Duration::from_micros(16667); // ~60 FPS
        let mut proximo_frame = Instant::now();

        let mut prev_f5 = false;
        let mut prev_tab = false;
        let mut prev_space = false;
        let mut prev_n = false;
        let mut prev_l = false;

        let mut msg: MSG = std::mem::zeroed();

        while en_ejecucion.load(Ordering::Relaxed) {
            // Procesar mensajes de Windows
            while PeekMessageW(&mut msg, std::ptr::null_mut(), 0, 0, PM_REMOVE) != 0 {
                if msg.message == WM_QUIT || msg.message == WM_CLOSE {
                    en_ejecucion.store(false, Ordering::Relaxed);
                    break;
                }
                TranslateMessage(&msg);
                DispatchMessageW(&msg);
            }

            if IsWindow(hwnd) == 0 {
                break;
            }

            // Tecla ESC para salir
            if (GetAsyncKeyState(VK_ESCAPE as i32) as u16 & 0x8000) != 0 {
                break;
            }

            // Atajos especiales
            let f5_down = (GetAsyncKeyState(VK_F5 as i32) as u16 & 0x8000) != 0;
            let forzar_f5 = f5_down && !prev_f5;
            prev_f5 = f5_down;

            let tab_down = (GetAsyncKeyState(VK_TAB as i32) as u16 & 0x8000) != 0;
            if tab_down && !prev_tab {
                resaltar_dif = !resaltar_dif;
            }
            prev_tab = tab_down;

            let l_down = (GetAsyncKeyState(b'L' as i32) as u16 & 0x8000) != 0;
            if l_down && !prev_l {
                ver_ref_fin = !ver_ref_fin;
                println!(
                    "[Visor] Referencia izquierda cambiada a: {}",
                    if ver_ref_fin { "ESTADO FIN DE CUADRO" } else { "ESTADO POR LÍNEA (emulador)" }
                );
            }
            prev_l = l_down;

            let space_down = (GetAsyncKeyState(VK_SPACE as i32) as u16 & 0x8000) != 0;
            if space_down && !prev_space {
                pausado = !pausado;
                println!("[Visor] Emulación {}", if pausado { "PAUSADA (usa N para avanzar)" } else { "REANUDADA" });
            }
            prev_space = space_down;

            let n_down = (GetAsyncKeyState(b'N' as i32) as u16 & 0x8000) != 0;
            let paso_n = n_down && !prev_n;
            prev_n = n_down;

            let shift_down = (GetAsyncKeyState(VK_SHIFT as i32) as u16 & 0x8000) != 0;

            if pausado && paso_n {
                paso_unico = true;
            }

            // Actualizar entrada de controles GBA (bits en 0 = presionado)
            let mut keys: u16 = 0x03FF;
            if (GetAsyncKeyState(VK_UP as i32) as u16 & 0x8000) != 0 { keys &= !(1 << 6); }
            if (GetAsyncKeyState(VK_DOWN as i32) as u16 & 0x8000) != 0 { keys &= !(1 << 7); }
            if (GetAsyncKeyState(VK_LEFT as i32) as u16 & 0x8000) != 0 { keys &= !(1 << 5); }
            if (GetAsyncKeyState(VK_RIGHT as i32) as u16 & 0x8000) != 0 { keys &= !(1 << 4); }
            if (GetAsyncKeyState(b'Z' as i32) as u16 & 0x8000) != 0 { keys &= !(1 << 0); }      // A
            if (GetAsyncKeyState(b'X' as i32) as u16 & 0x8000) != 0 { keys &= !(1 << 1); }      // B
            if (GetAsyncKeyState(VK_RETURN as i32) as u16 & 0x8000) != 0 { keys &= !(1 << 3); } // Start
            if (GetAsyncKeyState(VK_BACK as i32) as u16 & 0x8000) != 0 { keys &= !(1 << 2); }   // Select
            if (GetAsyncKeyState(b'A' as i32) as u16 & 0x8000) != 0 { keys &= !(1 << 9); }      // L
            if (GetAsyncKeyState(b'S' as i32) as u16 & 0x8000) != 0 { keys &= !(1 << 8); }      // R

            maquina.bus.keys = keys;

            // Avanzar emulación si no está pausado o si se pidió un paso único
            let debe_avanzar = !pausado || paso_unico;
            if debe_avanzar {
                paso_unico = false;

                // Capturar el estado por línea para diagnóstico
                maquina.bus.capturas_linea.clear();
                maquina.bus.capturar_linea = true;

                maquina.run_frame(5_000_000);
                maquina.bus.capturar_linea = false;

                cuadro += 1;
                cuadros_en_segundo += 1;

                // Renderizar PPU nativa con LUT vertical (registros y paleta) si hay capturas por línea
                let hay_lineas = maquina.bus.capturas_linea.len() == ALTO;
                let mut io_lineas = Box::new([[0u8; 0x60]; ALTO]);
                let mut paleta_lineas = Box::new([[0u8; 1024]; ALTO]);
                if hay_lineas {
                    for (idx, cap) in maquina.bus.capturas_linea.iter().enumerate() {
                        let l_io = cap.io.len().min(0x60);
                        io_lineas[idx][..l_io].copy_from_slice(&cap.io[..l_io]);
                        let l_pal = cap.palette.len().min(1024);
                        paleta_lineas[idx][..l_pal].copy_from_slice(&cap.palette[..l_pal]);
                    }
                }
                let nat_rgb = gpu.renderiza_con_lineas(
                    &maquina.bus.vram,
                    &maquina.bus.oam,
                    &maquina.bus.palette,
                    &maquina.bus.io,
                    if hay_lineas { Some(&*io_lineas) } else { None },
                    if hay_lineas { Some(&*paleta_lineas) } else { None },
                );
                let sw_linea_rgb = software_a_rgb(&maquina.bus.framebuffer);

                let dispcnt = io16(&maquina.bus.io, 0x00);
                let bldcnt = io16(&maquina.bus.io, 0x50);

                let diff_linea = diagnostico::diferencias(&sw_linea_rgb, &nat_rgb);
                let dif_linea_count = diff_linea.as_ref().map_or(0, |d| d.distintos);
                let caja_linea = diff_linea.as_ref().map_or((0, 0, 0, 0), |d| d.caja);

                // Calcular sw_fin solo si se necesita mostrar en pantalla o si hay sospecha de diferencia
                let sw_present_rgb = if ver_ref_fin {
                    render_software_fin(&maquina.bus.rom, &maquina.bus.vram, &maquina.bus.oam, &maquina.bus.palette, &maquina.bus.io)
                } else {
                    sw_linea_rgb.clone()
                };

                // Comprobar captura y diagnóstico usando evaluar_cuadro_en_vivo
                let dif_fin_est = if ver_ref_fin {
                    diagnostico::diferencias(&sw_present_rgb, &nat_rgb).map_or(0, |d| d.distintos)
                } else {
                    0
                };

                if let Some(motivo) = evaluar_cuadro_en_vivo(
                    cuadro,
                    dif_fin_est,
                    dif_linea_count,
                    caja_linea,
                    dispcnt,
                    bldcnt,
                    forzar_f5,
                    &mut filtro,
                ) {
                    capturas_totales += 1;
                    let tarea = TareaDiagnostico {
                        cuadro,
                        motivo,
                        sw_linea_rgb: sw_linea_rgb.clone(),
                        nat_rgb: nat_rgb.clone(),
                        vram: maquina.bus.vram.clone(),
                        oam: maquina.bus.oam.clone(),
                        palette: maquina.bus.palette.clone(),
                        io: maquina.bus.io.clone(),
                        capturas_linea: maquina.bus.capturas_linea.clone(),
                    };
                    let _ = tx_diag.try_send(tarea);
                }

                // Generar fotograma de presentación con la referencia elegida
                let buffer_bgrx = lienzo_doble_bgrx(&sw_present_rgb, &nat_rgb, resaltar_dif);

                // Dibujar en ventana con StretchDIBits (escalado limpio)
                let mut client_rect: RECT = std::mem::zeroed();
                GetClientRect(hwnd, &mut client_rect);
                let ancho_dst = client_rect.right - client_rect.left;
                let alto_dst = client_rect.bottom - client_rect.top;

                let mut bmi: BITMAPINFO = std::mem::zeroed();
                bmi.bmiHeader.biSize = std::mem::size_of::<BITMAPINFOHEADER>() as u32;
                bmi.bmiHeader.biWidth = ANCHO_DOBLE as i32;
                bmi.bmiHeader.biHeight = -(ALTO as i32); // Negativo para orden top-down
                bmi.bmiHeader.biPlanes = 1;
                bmi.bmiHeader.biBitCount = 32;
                bmi.bmiHeader.biCompression = BI_RGB;

                SetStretchBltMode(hdc, COLORONCOLOR as i32);
                StretchDIBits(
                    hdc,
                    0,
                    0,
                    ancho_dst,
                    alto_dst,
                    0,
                    0,
                    ANCHO_DOBLE as i32,
                    ALTO as i32,
                    buffer_bgrx.as_ptr() as *const _,
                    &bmi,
                    DIB_RGB_COLORS,
                    SRCCOPY,
                );

                // Calcular FPS y actualizar título cada 500 ms
                if ultimo_fps.elapsed() >= Duration::from_millis(500) {
                    let transcurrido = ultimo_fps.elapsed().as_secs_f32();
                    fps_actual = cuadros_en_segundo as f32 / transcurrido;
                    cuadros_en_segundo = 0;
                    ultimo_fps = Instant::now();

                    let modo_ref = if ver_ref_fin { "FIN CUADRO" } else { "LINEA (EMU)" };
                    let texto_titulo = format!(
                        "PPU Doble | Cuadro {} | {:.1} FPS | [L] Ref: {} | dif_linea: {} px | capturas: {} | [Tab] resaltado: {}\0",
                        cuadro,
                        fps_actual,
                        modo_ref,
                        dif_linea_count,
                        capturas_totales,
                        if resaltar_dif { "ON" } else { "OFF" }
                    );
                    let wide_titulo = encode_wide(&texto_titulo);
                    SetWindowTextW(hwnd, wide_titulo.as_ptr());
                }
            }

            // Sincronización a 60 FPS (si no está activo el turbo)
            if !shift_down && !pausado {
                let ahora = Instant::now();
                if ahora < proximo_frame {
                    let restante = proximo_frame - ahora;
                    if restante > Duration::from_millis(1) {
                        thread::sleep(restante - Duration::from_millis(1));
                    }
                    while Instant::now() < proximo_frame {
                        std::hint::spin_loop();
                    }
                }
                proximo_frame = Instant::now() + frame_duration;
            } else if pausado {
                thread::sleep(Duration::from_millis(16));
            }
        }

        ReleaseDC(hwnd, hdc);
        DestroyWindow(hwnd);
    }

    en_ejecucion.store(false, Ordering::Relaxed);
    let _ = hilo_diag.join();
    println!("\n[Visor] Sesión finalizada.");
}

#[cfg(windows)]
fn encode_wide(s: &str) -> Vec<u16> {
    s.encode_utf16().collect()
}
