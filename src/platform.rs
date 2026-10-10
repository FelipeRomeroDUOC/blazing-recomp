//! Plataforma Android (solo compila con `cfg(target_os = "android")`):
//! ciclo de vida NativeActivity, presentación GPU, entrada táctil con
//! háptica y logging a Logcat.

use std::sync::Arc;

use android_activity::input::{InputEvent, MotionEvent, MotionAction, KeyEvent, KeyAction, Keycode};
use android_activity::{InputStatus, MainEvent, PollEvent};
use crate::runner::GameRunner;
use raw_window_handle::{
    AndroidDisplayHandle, DisplayHandle, HandleError, HasDisplayHandle,
    HasWindowHandle, RawDisplayHandle,
};
use screen::ScreenKind;
use jni::{jni_sig, jni_str};

use crate::input::TouchController;
use crate::overlay::{build_elems_with_orientation, build_globals};

/// Dimensiones del framebuffer de la GBA (3:2).
const GBA_W: u32 = 240;
const GBA_H: u32 = 160;
const GBA_PIXELS: usize = (GBA_W * GBA_H) as usize;

/// Duración de la vibración háptica de pulsación, en ms (~35 ms permite
/// vencer la inercia mecánica de motores ERM en tablets sin demorar el feedback).
const HAPTIC_MS: i64 = 35;

/// Envoltorio seguro de la ventana nativa Android para `wgpu`.
///
/// El `NativeWindow` interno es dueño de su referencia adquirida: se
/// libera al descartar esta estructura.
struct AndroidWindow {
    window: android_activity::ndk::native_window::NativeWindow,
}

impl AndroidWindow {
    fn new(window: android_activity::ndk::native_window::NativeWindow) -> AndroidWindow {
        AndroidWindow { window }
    }

    fn native_window(&self) -> &android_activity::ndk::native_window::NativeWindow {
        &self.window
    }

    /// Tamaño físico de la superficie en píxeles.
    fn size(&self) -> (u32, u32) {
        (
            self.window.width().max(0) as u32,
            self.window.height().max(0) as u32,
        )
    }
}

impl HasWindowHandle for AndroidWindow {
    fn window_handle(&self) -> Result<raw_window_handle::WindowHandle<'_>, HandleError> {
        // SAFETY: el puntero `ANativeWindow` es válido mientras esta
        // estructura exista (el `NativeWindow` retiene su referencia) y no
        // cambia durante la vida del handle prestado.
        let raw = raw_window_handle::RawWindowHandle::AndroidNdk(
            raw_window_handle::AndroidNdkWindowHandle::new(
                self.window.ptr().cast::<std::ffi::c_void>(),
            ),
        );
        Ok(unsafe { raw_window_handle::WindowHandle::borrow_raw(raw) })
    }
}

impl HasDisplayHandle for AndroidWindow {
    fn display_handle(&self) -> Result<DisplayHandle<'_>, HandleError> {
        // SAFETY: `AndroidDisplayHandle` no contiene punteros prestados
        // (Android es un display único), según documenta raw-window-handle.
        let raw = RawDisplayHandle::Android(AndroidDisplayHandle::new());
        Ok(unsafe { DisplayHandle::borrow_raw(raw) })
    }
}

/// Compositor GPU unificado para Android: compone el marco de la GBA
/// (240x160 letterbox 3:2) y el overlay táctil procedural en una sola pasada.
struct Compositor {
    surface: wgpu::Surface<'static>,
    device: wgpu::Device,
    queue: wgpu::Queue,
    config: wgpu::SurfaceConfiguration,

    // Pipeline 1: Game Blit
    game_pipeline: wgpu::RenderPipeline,
    game_bind_group: wgpu::BindGroup,
    game_texture: wgpu::Texture,
    game_params_buffer: wgpu::Buffer,

    // Pipeline 2: Overlay táctil
    overlay_pipeline: wgpu::RenderPipeline,
    overlay_bind_group: wgpu::BindGroup,
    overlay_globals_buffer: wgpu::Buffer,
    overlay_elems_buffer: wgpu::Buffer,
    _overlay_font_texture: wgpu::Texture,

    // Pipeline 3: HUD / Contador de FPS
    hud_pipeline: wgpu::RenderPipeline,
    hud_bind_group: wgpu::BindGroup,
    hud_texture: wgpu::Texture,
    hud_params_buffer: wgpu::Buffer,
    fps_counter: crate::fps::FpsCounter,
}

fn request_adapter_fallback(
    instance: &wgpu::Instance,
    surface: &wgpu::Surface<'_>,
) -> Result<wgpu::Adapter, String> {
    if let Ok(a) = pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
        power_preference: wgpu::PowerPreference::LowPower,
        compatible_surface: Some(surface),
        force_fallback_adapter: false,
    })) {
        return Ok(a);
    }

    if let Ok(a) = pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
        power_preference: wgpu::PowerPreference::HighPerformance,
        compatible_surface: Some(surface),
        force_fallback_adapter: false,
    })) {
        return Ok(a);
    }

    if let Ok(a) = pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
        power_preference: wgpu::PowerPreference::None,
        compatible_surface: Some(surface),
        force_fallback_adapter: true,
    })) {
        return Ok(a);
    }

    pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
        power_preference: wgpu::PowerPreference::None,
        compatible_surface: None,
        force_fallback_adapter: false,
    }))
    .map_err(|e| format!("no graphics adapter: {e}"))
}

fn create_gpu_context(
    window: Arc<AndroidWindow>,
) -> Result<(wgpu::Surface<'static>, wgpu::Adapter), String> {
    // 1. Intentar Vulkan primero (bajo overhead de CPU y sincronización directa por hardware)
    let vk_instance = wgpu::Instance::new(&wgpu::InstanceDescriptor {
        backends: wgpu::Backends::VULKAN,
        ..Default::default()
    });
    if let Ok(vk_surface) = vk_instance.create_surface(window.clone()) {
        if let Ok(vk_adapter) = request_adapter_fallback(&vk_instance, &vk_surface) {
            log::info!("Adaptador GPU inicializado exitosamente con Vulkan nativo");
            return Ok((vk_surface, vk_adapter));
        }
    }

    log::warn!("Backend Vulkan no pudo inicializar la superficie o adaptador. Probando OpenGL ES como fallback...");

    // 2. Fallback a OpenGL ES (compatibilidad en emuladores o dispositivos sin Vulkan)
    let gl_instance = wgpu::Instance::new(&wgpu::InstanceDescriptor {
        backends: wgpu::Backends::GL,
        ..Default::default()
    });
    let gl_surface = gl_instance
        .create_surface(window.clone())
        .map_err(|e| format!("surface OpenGL ES: {e}"))?;
    let gl_adapter = request_adapter_fallback(&gl_instance, &gl_surface)
        .map_err(|e| format!("no se encontró adaptador GPU compatible ni en Vulkan ni en GLES: {e}"))?;

    log::info!("Adaptador GPU inicializado exitosamente con OpenGL ES");
    Ok((gl_surface, gl_adapter))
}

impl Compositor {
    fn new(window: Arc<AndroidWindow>, vsync: bool) -> Result<Compositor, String> {
        let (surface, adapter) = create_gpu_context(window.clone())?;

        let (device, queue) = pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor {
            label: Some("blazing-compositor"),
            required_limits: adapter.limits(),
            ..Default::default()
        }))
        .map_err(|e| format!("graphics device: {e}"))?;

        let caps = surface.get_capabilities(&adapter);
        let (format, alpha_mode) = crate::error_screen::select_surface_format_and_alpha(
            &caps.formats,
            &caps.alpha_modes,
        );

        let (w, h) = window.size();
        let (w, h) = (w.max(1), h.max(1));
        let present_mode = if vsync {
            if caps.present_modes.contains(&wgpu::PresentMode::Mailbox) {
                log::info!("PresentMode seleccionado: Mailbox (triple buffering libre de bloqueos)");
                wgpu::PresentMode::Mailbox
            } else if caps.present_modes.contains(&wgpu::PresentMode::FifoRelaxed) {
                log::info!("PresentMode seleccionado: FifoRelaxed (VSync adaptable sin caídas)");
                wgpu::PresentMode::FifoRelaxed
            } else {
                log::info!("PresentMode seleccionado: AutoVsync (FIFO estándar)");
                wgpu::PresentMode::AutoVsync
            }
        } else {
            wgpu::PresentMode::AutoNoVsync
        };

        let config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format,
            width: w,
            height: h,
            present_mode,
            alpha_mode,
            view_formats: vec![],
            desired_maximum_frame_latency: 1,
        };
        surface.configure(&device, &config);

        // 1. Game Blit Pipeline
        let game_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("game-blit-shader"),
            source: wgpu::ShaderSource::Wgsl(crate::overlay_shader::GAME_BLIT_WGSL.into()),
        });
        let game_bgl = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("game-blit-bgl"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 2,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
            ],
        });
        let game_pl = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("game-blit-pl"),
            bind_group_layouts: &[&game_bgl],
            push_constant_ranges: &[],
        });
        let game_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("game-blit-pipeline"),
            layout: Some(&game_pl),
            vertex: wgpu::VertexState {
                module: &game_shader,
                entry_point: Some("vs_main"),
                compilation_options: Default::default(),
                buffers: &[],
            },
            fragment: Some(wgpu::FragmentState {
                module: &game_shader,
                entry_point: Some("fs_main"),
                compilation_options: Default::default(),
                targets: &[Some(wgpu::ColorTargetState {
                    format,
                    blend: None,
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            primitive: wgpu::PrimitiveState::default(),
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            multiview: None,
            cache: None,
        });

        let game_texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("game-texture"),
            size: wgpu::Extent3d {
                width: GBA_W,
                height: GBA_H,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8Unorm,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });
        let game_texture_view = game_texture.create_view(&wgpu::TextureViewDescriptor::default());
        let game_sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("game-sampler"),
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Nearest,
            min_filter: wgpu::FilterMode::Nearest,
            mipmap_filter: wgpu::FilterMode::Nearest,
            ..Default::default()
        });
        let game_params_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("game-params-buffer"),
            size: 32,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let game_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("game-bind-group"),
            layout: &game_bgl,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::Sampler(&game_sampler),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::TextureView(&game_texture_view),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: game_params_buffer.as_entire_binding(),
                },
            ],
        });

        // 2. Overlay Pipeline
        let overlay_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("overlay-shader"),
            source: wgpu::ShaderSource::Wgsl(crate::overlay_shader::WGSL.into()),
        });
        let overlay_bgl = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("overlay-bgl"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 2,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 3,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
            ],
        });
        let overlay_pl = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("overlay-pl"),
            bind_group_layouts: &[&overlay_bgl],
            push_constant_ranges: &[],
        });
        let overlay_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("overlay-pipeline"),
            layout: Some(&overlay_pl),
            vertex: wgpu::VertexState {
                module: &overlay_shader,
                entry_point: Some("vs_main"),
                compilation_options: Default::default(),
                buffers: &[],
            },
            fragment: Some(wgpu::FragmentState {
                module: &overlay_shader,
                entry_point: Some("fs_main"),
                compilation_options: Default::default(),
                targets: &[Some(wgpu::ColorTargetState {
                    format,
                    blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            primitive: wgpu::PrimitiveState::default(),
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            multiview: None,
            cache: None,
        });

        let overlay_globals_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("overlay-globals-buffer"),
            size: 16,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let overlay_elems_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("overlay-elems-buffer"),
            size: 768, // 12 elems * 64 B (16 floats)
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let font_atlas = crate::overlay::build_font_atlas();
        let overlay_font_texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("overlay-font-texture"),
            size: wgpu::Extent3d {
                width: font_atlas.width,
                height: font_atlas.height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::R8Unorm,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });
        queue.write_texture(
            wgpu::TexelCopyTextureInfo {
                texture: &overlay_font_texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            &font_atlas.data,
            wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(font_atlas.width),
                rows_per_image: Some(font_atlas.height),
            },
            wgpu::Extent3d {
                width: font_atlas.width,
                height: font_atlas.height,
                depth_or_array_layers: 1,
            },
        );
        let overlay_font_view = overlay_font_texture.create_view(&wgpu::TextureViewDescriptor::default());
        let overlay_font_sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("overlay-font-sampler"),
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            mipmap_filter: wgpu::FilterMode::Nearest,
            ..Default::default()
        });

        let overlay_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("overlay-bind-group"),
            layout: &overlay_bgl,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: overlay_globals_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: overlay_elems_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: wgpu::BindingResource::TextureView(&overlay_font_view),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: wgpu::BindingResource::Sampler(&overlay_font_sampler),
                },
            ],
        });

        // 3. HUD Pipeline (Contador de FPS)
        let hud_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("hud-shader"),
            source: wgpu::ShaderSource::Wgsl(crate::overlay_shader::HUD_BLIT_WGSL.into()),
        });
        let hud_bgl = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("hud-bgl"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 2,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
            ],
        });
        let hud_pl = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("hud-pl"),
            bind_group_layouts: &[&hud_bgl],
            push_constant_ranges: &[],
        });
        let hud_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("hud-pipeline"),
            layout: Some(&hud_pl),
            vertex: wgpu::VertexState {
                module: &hud_shader,
                entry_point: Some("vs_main"),
                compilation_options: Default::default(),
                buffers: &[],
            },
            fragment: Some(wgpu::FragmentState {
                module: &hud_shader,
                entry_point: Some("fs_main"),
                compilation_options: Default::default(),
                targets: &[Some(wgpu::ColorTargetState {
                    format,
                    blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            primitive: wgpu::PrimitiveState::default(),
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            multiview: None,
            cache: None,
        });

        let mut fps_counter = crate::fps::FpsCounter::new()
            .map_err(|e| format!("fps counter init: {e}"))?;
        let backend_name = match adapter.get_info().backend {
            wgpu::Backend::Vulkan => "Vulkan",
            _ => "OpenGL",
        };
        fps_counter.set_renderer(backend_name);

        let (hud_w, hud_h) = fps_counter.dimensions();
        let hud_texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("hud-fps-texture"),
            size: wgpu::Extent3d {
                width: hud_w,
                height: hud_h,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8Unorm,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });

        let hud_rgba_bytes = unsafe {
            std::slice::from_raw_parts(
                fps_counter.badge_rgba().as_ptr().cast(),
                hud_w as usize * hud_h as usize * 4,
            )
        };
        queue.write_texture(
            wgpu::TexelCopyTextureInfo {
                texture: &hud_texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            hud_rgba_bytes,
            wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(hud_w * 4),
                rows_per_image: Some(hud_h),
            },
            wgpu::Extent3d {
                width: hud_w,
                height: hud_h,
                depth_or_array_layers: 1,
            },
        );
        fps_counter.mark_clean();

        let hud_texture_view = hud_texture.create_view(&wgpu::TextureViewDescriptor::default());
        let hud_sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("hud-sampler"),
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            ..Default::default()
        });
        let hud_params_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("hud-params-buffer"),
            size: 32, // 8 x f32
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let hud_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("hud-bind-group"),
            layout: &hud_bgl,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::Sampler(&hud_sampler),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::TextureView(&hud_texture_view),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: hud_params_buffer.as_entire_binding(),
                },
            ],
        });

        Ok(Compositor {
            surface,
            device,
            queue,
            config,
            game_pipeline,
            game_bind_group,
            game_texture,
            game_params_buffer,
            overlay_pipeline,
            overlay_bind_group,
            overlay_globals_buffer,
            overlay_elems_buffer,
            _overlay_font_texture: overlay_font_texture,
            hud_pipeline,
            hud_bind_group,
            hud_texture,
            hud_params_buffer,
            fps_counter,
        })
    }

    fn render(
        &mut self,
        rgba: &[[u8; 4]],
        window_physical: (u32, u32),
        touch: &TouchController,
        is_native: bool,
    ) -> Result<(f64, f64), String> {
        let t_render_start = std::time::Instant::now();
        let (w, h) = (window_physical.0.max(1), window_physical.1.max(1));
        if (w, h) != (self.config.width, self.config.height) {
            self.config.width = w;
            self.config.height = h;
            self.surface.configure(&self.device, &self.config);
        }

        // 1. Upload frame GBA
        debug_assert_eq!(rgba.len(), GBA_PIXELS);
        let rgba_bytes = unsafe { std::slice::from_raw_parts(rgba.as_ptr().cast(), rgba.len() * 4) };
        self.queue.write_texture(
            wgpu::TexelCopyTextureInfo {
                texture: &self.game_texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            rgba_bytes,
            wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(GBA_W * 4),
                rows_per_image: Some(GBA_H),
            },
            wgpu::Extent3d {
                width: GBA_W,
                height: GBA_H,
                depth_or_array_layers: 1,
            },
        );

        // 2. Colocación del marco del juego (3:2)
        // Portrait: mitad superior (hasta 0.48 de la altura)
        // Landscape: centrado
        let (origin_x, origin_y, dst_w, dst_h) = if w < h {
            let available_h = (h as f32) * 0.48;
            let scale = ((w as f32) / (GBA_W as f32)).min(available_h / (GBA_H as f32));
            let dw = (GBA_W as f32) * scale;
            let dh = (GBA_H as f32) * scale;
            let ox = ((w as f32) - dw) * 0.5;
            let oy = (available_h - dh) * 0.5;
            (ox, oy, dw, dh)
        } else {
            let scale = ((w as f32) / (GBA_W as f32)).min((h as f32) / (GBA_H as f32));
            let dw = (GBA_W as f32) * scale;
            let dh = (GBA_H as f32) * scale;
            let ox = ((w as f32) - dw) * 0.5;
            let oy = ((h as f32) - dh) * 0.5;
            (ox, oy, dw, dh)
        };

        let game_params: [f32; 8] = [
            origin_x,
            origin_y,
            dst_w,
            dst_h,
            w as f32,
            h as f32,
            0.0,
            0.0,
        ];
        self.queue.write_buffer(
            &self.game_params_buffer,
            0,
            bytemuck::cast_slice(&game_params),
        );

        // 3. Parámetros del overlay
        let mut globals = build_globals(w, h, touch.orientation());
        globals.shell = if w < h { 1.0 } else { 0.0 };
        let globals_packed = globals.pack();
        self.queue.write_buffer(
            &self.overlay_globals_buffer,
            0,
            bytemuck::cast_slice(&globals_packed),
        );

        let elems = build_elems_with_orientation(touch.areas(), touch.orientation(), |b| touch.is_pressed(b));
        let mut elems_packed = [0.0f32; 12 * 16];
        for (i, elem) in elems.iter().take(12).enumerate() {
            let p = elem.pack();
            elems_packed[i * 16..(i + 1) * 16].copy_from_slice(&p);
        }
        self.queue.write_buffer(
            &self.overlay_elems_buffer,
            0,
            bytemuck::cast_slice(&elems_packed),
        );

        // 4. Actualizar telemetría y textura del HUD de FPS
        self.fps_counter.set_native(is_native);
        if self.fps_counter.is_dirty() {
            let (hud_w, hud_h) = self.fps_counter.dimensions();
            let hud_rgba_bytes = unsafe {
                std::slice::from_raw_parts(
                    self.fps_counter.badge_rgba().as_ptr().cast(),
                    hud_w as usize * hud_h as usize * 4,
                )
            };
            self.queue.write_texture(
                wgpu::TexelCopyTextureInfo {
                    texture: &self.hud_texture,
                    mip_level: 0,
                    origin: wgpu::Origin3d::ZERO,
                    aspect: wgpu::TextureAspect::All,
                },
                hud_rgba_bytes,
                wgpu::TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(hud_w * 4),
                    rows_per_image: Some(hud_h),
                },
                wgpu::Extent3d {
                    width: hud_w,
                    height: hud_h,
                    depth_or_array_layers: 1,
                },
            );
            self.fps_counter.mark_clean();
        }

        // Posición del badge de FPS: márgenes seguros de Safe Area contra esquinas redondeadas y notch
        let (hud_w, hud_h) = self.fps_counter.dimensions();
        let margin_x = if w > h { 96.0f32 } else { 24.0f32 };
        let margin_y = if w > h { 24.0f32 } else { 44.0f32 };
        let hud_ox = (w as f32) - (hud_w as f32) - margin_x;
        let hud_oy = margin_y;

        let hud_params: [f32; 8] = [
            hud_ox.max(0.0),
            hud_oy.max(0.0),
            hud_w as f32,
            hud_h as f32,
            w as f32,
            h as f32,
            0.0,
            0.0,
        ];
        self.queue.write_buffer(
            &self.hud_params_buffer,
            0,
            bytemuck::cast_slice(&hud_params),
        );

        // 5. Render pass en el swapchain
        let t_wait_start = std::time::Instant::now();
        let frame = match self.surface.get_current_texture() {
            Ok(f) => f,
            Err(wgpu::SurfaceError::Outdated | wgpu::SurfaceError::Lost | wgpu::SurfaceError::OutOfMemory) => {
                self.surface.configure(&self.device, &self.config);
                self.surface
                    .get_current_texture()
                    .map_err(|e| e.to_string())?
            }
            Err(e) => return Err(e.to_string()),
        };
        let espera_ms = t_wait_start.elapsed().as_secs_f64() * 1000.0;
        let view = frame
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());

        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("compositor-pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    resolve_target: None,
                    depth_slice: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
            });

            // Pase 1: Marco de juego
            pass.set_pipeline(&self.game_pipeline);
            pass.set_bind_group(0, &self.game_bind_group, &[]);
            pass.draw(0..3, 0..1);

            // Pase 2: Overlay táctil y carcasa (con alpha blending)
            if touch.touch_enabled {
                pass.set_pipeline(&self.overlay_pipeline);
                pass.set_bind_group(0, &self.overlay_bind_group, &[]);
                pass.draw(0..3, 0..1);
            }

            // Pase 3: HUD / Contador de FPS (siempre visible con tipografía de assets)
            pass.set_pipeline(&self.hud_pipeline);
            pass.set_bind_group(0, &self.hud_bind_group, &[]);
            pass.draw(0..3, 0..1);
        }

        self.queue.submit([encoder.finish()]);
        frame.present();
        let dibujo_ms = t_render_start.elapsed().as_secs_f64() * 1000.0;
        Ok((dibujo_ms, espera_ms))
    }
}

/// Respuesta háptica por vibración, vía JNI sobre el servicio `Vibrator`
/// del sistema.
///
/// Seguridad (contexto de la propuesta): toda llamada JNI pasa por
/// `attach_current_thread`, que captura las excepciones Java y las
/// devuelve como `Error::CaughtJavaException`; cualquier fallo (sin motor,
/// sin permiso, sin JVM) se registra y es un no-op silencioso que nunca
/// detiene el bucle de la aplicación.
struct Haptics {
    vm: jni::JavaVM,
    /// Objeto Activity de la NativeActivity (referencia global).
    activity: jni::objects::Global<jni::objects::JObject<'static>>,
}

impl Haptics {
    /// Conecta con la JVM y la Activity entregadas por `android-activity`.
    /// `None` si el entorno no ofrece JNI (háptica desactivada).
    fn connect(
        vm_ptr: *mut std::ffi::c_void,
        activity_ptr: *mut std::ffi::c_void,
    ) -> Option<Haptics> {
        if vm_ptr.is_null() || activity_ptr.is_null() {
            log::info!("haptics: sin punteros JNI; vibración desactivada");
            return None;
        }
        // SAFETY: android-activity garantiza que `vm_as_ptr()` es la
        // `JavaVM` válida y `activity_as_ptr()` el jobject de la
        // NativeActivity viva durante la llamada.
        let vm = unsafe { jni::JavaVM::from_raw(vm_ptr.cast()) };
        let activity = vm
            .attach_current_thread(|env| -> Result<_, jni::errors::Error> {
                // SAFETY: puntero de Activity válido entregado arriba.
                let obj =
                    unsafe { jni::objects::JObject::from_raw(env, activity_ptr.cast()) };
                let global = env.new_global_ref(&obj)?;
                // El handle original es una referencia global mantenida
                // por android-activity, no un local: no se borra como
                // local (DeleteLocalRef sobre un global sería inválido).
                std::mem::forget(obj);
                Ok(global)
            })
            .ok()?;
        log::info!("haptics: conectado al servicio del sistema");
        Some(Haptics { vm, activity })
    }

    /// Vibración de `ms` milisegundos. No-op ante cualquier error.
    fn vibrate(&self, ms: i64) {
        let run = |env: &mut jni::Env| -> Result<(), jni::errors::Error> {
            // `Object Context.VIBRATOR_SERVICE` (String constante desde
            // API 1; uniforme en toda la gama objetivo del port).
            let service = env
                .get_static_field(
                    jni_str!("android/content/Context"),
                    jni_str!("VIBRATOR_SERVICE"),
                    jni_sig!("Ljava/lang/String;"),
                )?
                .l()?;
            let vibrator = env
                .call_method(
                    self.activity.as_obj(),
                    jni_str!("getSystemService"),
                    jni_sig!("(Ljava/lang/String;)Ljava/lang/Object;"),
                    &[jni::objects::JValue::Object(&service)],
                )?
                .l()?;
            if vibrator.is_null() {
                return Ok(()); // sin motor de vibración: no-op
            }

            // Intento 1: API 26+ (Android 8.0 a 15+).
            // `VibrationEffect.createOneShot(ms, DEFAULT_AMPLITUDE)` asegura
            // que Android 11+ clasifique la vibración como feedback háptico
            // y no la descarte por falta de atributos.
            let modern_vibrate = |env: &mut jni::Env| -> Result<(), jni::errors::Error> {
                let effect_class = env.find_class(jni_str!("android/os/VibrationEffect"))?;
                let effect = env
                    .call_static_method(
                        &effect_class,
                        jni_str!("createOneShot"),
                        jni_sig!("(JI)Landroid/os/VibrationEffect;"),
                        &[jni::objects::JValue::Long(ms), jni::objects::JValue::Int(-1)],
                    )?
                    .l()?;
                env.call_method(
                    &vibrator,
                    jni_str!("vibrate"),
                    jni_sig!("(Landroid/os/VibrationEffect;)V"),
                    &[jni::objects::JValue::Object(&effect)],
                )?;
                Ok(())
            };

            if let Err(e) = modern_vibrate(env) {
                // Si la excepción quedó pendiente en Java, la limpiamos antes del fallback
                env.exception_clear();
                log::debug!("haptics: VibrationEffect fallback ({e:?})");

                // Intento 2: Fallback API 1..25 `void vibrate(long)`.
                env.call_method(
                    &vibrator,
                    jni_str!("vibrate"),
                    jni_sig!("(J)V"),
                    &[jni::objects::JValue::Long(ms)],
                )?;
            }

            Ok(())
        };
        if let Err(e) = self.vm.attach_current_thread(run) {
            log::warn!("haptics: vibración no disponible ({e:?})");
        }
    }
}

/// Gestor JNI de permisos de almacenamiento en Android (Scoped Storage).
struct PermissionsManager {
    vm: jni::JavaVM,
    activity: jni::objects::Global<jni::objects::JObject<'static>>,
}

impl PermissionsManager {
    fn connect(
        vm_ptr: *mut std::ffi::c_void,
        activity_ptr: *mut std::ffi::c_void,
    ) -> Option<PermissionsManager> {
        if vm_ptr.is_null() || activity_ptr.is_null() {
            log::info!("permisos: sin punteros JNI; solicitud desactivada");
            return None;
        }
        let vm = unsafe { jni::JavaVM::from_raw(vm_ptr.cast()) };
        let activity = vm
            .attach_current_thread(|env| -> Result<_, jni::errors::Error> {
                let obj =
                    unsafe { jni::objects::JObject::from_raw(env, activity_ptr.cast()) };
                let global = env.new_global_ref(&obj)?;
                std::mem::forget(obj);
                Ok(global)
            })
            .ok()?;
        log::info!("permisos: conectado al contexto de la aplicación");
        Some(PermissionsManager { vm, activity })
    }

    /// Comprueba el estado de permisos y solicita acceso a almacenamiento si no está concedido.
    fn request_storage_permissions(&self) {
        let run = |env: &mut jni::Env| -> Result<(), jni::errors::Error> {
            let build_version_class = env.find_class(jni_str!("android/os/Build$VERSION"))?;
            let sdk_int = env
                .get_static_field(
                    &build_version_class,
                    jni_str!("SDK_INT"),
                    jni_sig!("I"),
                )?
                .i()?;

            log::info!("permisos: Android SDK_INT = {sdk_int}");

            // 1. Solicitar permisos de runtime estándar (para visibilidad en Ajustes de la App)
            let string_class = env.find_class(jni_str!("java/lang/String"))?;
            let perms = env.new_object_array(2, &string_class, jni::objects::JObject::null())?;
            let p_read = env.new_string("android.permission.READ_EXTERNAL_STORAGE")?;
            let p_write = env.new_string("android.permission.WRITE_EXTERNAL_STORAGE")?;
            perms.set_element(env, 0, &p_read)?;
            perms.set_element(env, 1, &p_write)?;

            let _ = env.call_method(
                self.activity.as_obj(),
                jni_str!("requestPermissions"),
                jni_sig!("([Ljava/lang/String;I)V"),
                &[jni::objects::JValue::Object(&perms), jni::objects::JValue::Int(1001)],
            );
            if env.exception_check() {
                env.exception_clear();
            }

            // 2. En Android 11+ (API 30+), verificar Environment.isExternalStorageManager()
            if sdk_int >= 30 {
                let env_class = env.find_class(jni_str!("android/os/Environment"))?;
                let is_manager = env
                    .call_static_method(
                        &env_class,
                        jni_str!("isExternalStorageManager"),
                        jni_sig!("()Z"),
                        &[],
                    )?
                    .z()?;

                if !is_manager {
                    log::info!("permisos: MANAGE_EXTERNAL_STORAGE pendiente; abriendo pantalla del sistema");
                    let intent_class = env.find_class(jni_str!("android/content/Intent"))?;
                    let action = env.new_string("android.settings.MANAGE_APP_ALL_FILES_ACCESS_PERMISSION")?;
                    let intent = env.new_object(
                        &intent_class,
                        jni_sig!("(Ljava/lang/String;)V"),
                        &[jni::objects::JValue::Object(&action)],
                    )?;

                    let uri_str = env.new_string("package:com.blazingrecomp.game")?;
                    let uri_class = env.find_class(jni_str!("android/net/Uri"))?;
                    let uri = env.call_static_method(
                        &uri_class,
                        jni_str!("parse"),
                        jni_sig!("(Ljava/lang/String;)Landroid/net/Uri;"),
                        &[jni::objects::JValue::Object(&uri_str)],
                    )?.l()?;

                    let _ = env.call_method(
                        &intent,
                        jni_str!("setData"),
                        jni_sig!("(Landroid/net/Uri;)Landroid/content/Intent;"),
                        &[jni::objects::JValue::Object(&uri)],
                    )?;

                    let start_res = env.call_method(
                        self.activity.as_obj(),
                        jni_str!("startActivity"),
                        jni_sig!("(Landroid/content/Intent;)V"),
                        &[jni::objects::JValue::Object(&intent)],
                    );

                    if start_res.is_err() {
                        env.exception_clear();
                        let action_generic = env.new_string("android.settings.MANAGE_ALL_FILES_ACCESS_PERMISSION")?;
                        let generic_intent = env.new_object(
                            &intent_class,
                            jni_sig!("(Ljava/lang/String;)V"),
                            &[jni::objects::JValue::Object(&action_generic)],
                        )?;
                        let _ = env.call_method(
                            self.activity.as_obj(),
                            jni_str!("startActivity"),
                            jni_sig!("(Landroid/content/Intent;)V"),
                            &[jni::objects::JValue::Object(&generic_intent)],
                        );
                        if env.exception_check() {
                            env.exception_clear();
                        }
                    }
                } else {
                    log::info!("permisos: MANAGE_EXTERNAL_STORAGE ya concedido");
                }
            }

            Ok(())
        };

        if let Err(e) = self.vm.attach_current_thread(run) {
            log::warn!("permisos: error comprobando o solicitando permisos JNI ({e:?})");
        }
    }

    /// Comprueba si el dispositivo es un Android TV (Leanback / UI_MODE_TYPE_TELEVISION).
    fn is_television(&self) -> bool {
        let run = |env: &mut jni::Env| -> Result<bool, jni::errors::Error> {
            let res_obj = env.call_method(
                self.activity.as_obj(),
                jni_str!("getResources"),
                jni_sig!("()Landroid/content/res/Resources;"),
                &[],
            )?.l()?;
            let conf_obj = env.call_method(
                &res_obj,
                jni_str!("getConfiguration"),
                jni_sig!("()Landroid/content/res/Configuration;"),
                &[],
            )?.l()?;
            let ui_mode = env.get_field(&conf_obj, jni_str!("uiMode"), jni_sig!("I"))?.i()?;
            let conf_class = env.find_class(jni_str!("android/content/res/Configuration"))?;
            let tv_mode = env.get_static_field(&conf_class, jni_str!("UI_MODE_TYPE_TELEVISION"), jni_sig!("I"))?.i()?;
            Ok((ui_mode & 0x0F) == tv_mode)
        };
        match self.vm.attach_current_thread(run) {
            Ok(is_tv) => is_tv,
            Err(_) => false,
        }
    }
}

fn init_perf_file(
    search_dirs: &[std::path::PathBuf],
    backend: &str,
    is_native: bool,
    clock_cost_ns: u64,
) -> Option<std::fs::File> {
    let candidate_dirs = crate::perf::resolve_perf_candidates(search_dirs);
    for dir in &candidate_dirs {
        let candidate = dir.join("blazing-perf.txt");
        if let Ok(mut f) = std::fs::File::create(&candidate) {
            use std::io::Write;
            let header = format!(
                "# backend={} modo={} reloj={}ns\n",
                backend,
                if is_native { "AOT" } else { "INT" },
                clock_cost_ns
            );
            if f.write_all(header.as_bytes()).is_ok() && f.flush().is_ok() {
                log::info!("Telemetría de rendimiento iniciada en {:?}", candidate);
                return Some(f);
            }
        }
    }
    log::warn!("No se pudo inicializar blazing-perf.txt en ningún directorio de almacenamiento");
    None
}

/// Estado de la aplicación: lo que persiste entre pausas y reanudaciones
/// del sistema Android.
struct AppState {
    /// Ejecutor del juego y modelo de consola. `None` hasta que se carguen
    /// la ROM y el BIOS del usuario con hashes verificados (AGENTS.md §3).
    runner: Option<GameRunner>,
    /// Compositor GPU unificado (`wgpu`) sobre la ventana Android. `None`
    /// cuando no hay ventana viva (`TerminateWindow`, `Pause`, `Destroy`).
    compositor: Option<Compositor>,
    /// Ventana propiedad del compositor, para consultar su tamaño físico.
    window: Option<Arc<AndroidWindow>>,
    /// Panel físico que se simula (perfil de `screen`).
    #[allow(dead_code)]
    screen_kind: ScreenKind,
    /// Búfer de respaldo para presentación cuando no hay juego activo.
    placeholder_rgba: Vec<[u8; 4]>,
    /// Controlador táctil (hitboxes, estado KEYINPUT, `touch_enabled`).
    touch: TouchController,
    /// Máscara de botones físicos / gamepad / control remoto de Android TV (activo en bajo, 0x03FF = reposo).
    physical_key_mask: u16,
    /// Háptica; `None` si no hay JNI disponible (no-op).
    haptics: Option<Haptics>,
    /// Error fatal retenido para presentación visual de diagnóstico.
    fatal_error: Option<String>,
    /// Renderizador tipográfico por software para la pantalla de error.
    error_renderer: Option<crate::error_screen::ErrorScreenRenderer>,
    /// Acumulador de telemetría de rendimiento por fase de fotograma.
    perf: crate::perf::FramePerf,
    /// Archivo para persistir las líneas de telemetría en Descargas / almacenamiento.
    perf_file: Option<std::fs::File>,
    /// Bandera para evitar saturar el log si la escritura falla.
    perf_write_warned: bool,
    /// Costo medido en nanosegundos de una lectura de reloj `Instant::now()`.
    clock_cost_ns: u64,
    /// Directorios candidatos para escribir `blazing-perf.txt`.
    search_dirs: Vec<std::path::PathBuf>,
}

impl AppState {
    fn new() -> AppState {
        let screen_kind = ScreenKind::Frontlit;
        AppState {
            runner: None,
            compositor: None,
            window: None,
            screen_kind,
            placeholder_rgba: vec![[0, 0, 0, 255]; GBA_PIXELS],
            touch: TouchController::new(),
            physical_key_mask: 0x03FF,
            haptics: None,
            fatal_error: None,
            error_renderer: None,
            perf: crate::perf::FramePerf::new(),
            perf_file: None,
            perf_write_warned: false,
            clock_cost_ns: crate::perf::measure_clock_cost_ns(),
            search_dirs: Vec::new(),
        }
    }

    /// Renderiza un mensaje de error por software en la superficie nativa de Android.
    fn render_software_error(&mut self, title: &str, message: &str) {
        let Some(window) = &self.window else { return };
        let (w, h) = window.size();
        if w == 0 || h == 0 {
            return;
        }
        if self.error_renderer.is_none() {
            self.error_renderer = crate::error_screen::ErrorScreenRenderer::new().ok();
        }
        let Some(renderer) = &self.error_renderer else { return };

        let native_win = window.native_window();
        if let Ok(mut guard) = native_win.lock(None) {
            let stride = guard.stride();
            let height = guard.height();
            let width = guard.width();
            let bpp = guard.format().bytes_per_pixel().unwrap_or(4);
            if let Some(bytes) = guard.bytes() {
                let slice: &mut [u8] = unsafe {
                    std::slice::from_raw_parts_mut(bytes.as_mut_ptr().cast(), bytes.len())
                };
                renderer.render(slice, width, height, stride, bpp, title, message);
            }
        }
    }

    /// Carga ROM y BIOS en el modelo de hardware de la consola (perfiles de
    /// AGENTS.md §3). Se invoca una vez verificados los hashes SHA-256.
    #[allow(dead_code)]
    fn load(&mut self, rom: Vec<u8>, bios: Option<&[u8]>) {
        self.runner = Some(GameRunner::new(rom, bios, self.screen_kind));
    }

    /// Libera los recursos GPU y de emulación al pausar; Android puede
    /// matar la NativeActivity sin aviso y conviene no sostener RAM del
    /// juego ni una superficie de una ventana que ya no existe.
    fn suspend(&mut self) {
        if self.compositor.take().is_some() || self.window.take().is_some() {
            log::debug!("recursos GPU liberados (suspend)");
        }
        if let Some(runner) = self.runner.take() {
            log::debug!("GameRunner liberado (suspend)");
            drop(runner);
        }
    }

    /// Destruye el compositor y su ventana (evento `TerminateWindow`).
    fn remove_compositor(&mut self) {
        self.compositor = None;
        self.window = None;
    }

    /// Crea el compositor GPU sobre `window`. Solo con ventana viva:
    /// llamarlo fuera de `InitWindow` dejaría el compositor con un puntero
    /// a ventana huérfana. Recalcula también las hitboxes táctiles.
    fn create_compositor(&mut self, window: Option<Arc<AndroidWindow>>) {
        self.remove_compositor();
        let Some(window) = window else {
            log::warn!("event: InitWindow sin ventana nativa");
            return;
        };
        let (w, h) = window.size();
        log::info!("ventana nativa {}x{}: creando compositor", w, h);
        // Auto-rotación: la geometría de controles sigue a la ventana.
        if w > 0 && h > 0 {
            self.touch.resize(w, h);
        }
        match Compositor::new(
            window.clone(),
            true, // vsync activo
        ) {
            Ok(c) => {
                self.window = Some(window);
                self.compositor = Some(c);
                self.fatal_error = None;
                log::info!("compositor GPU listo (juego + overlay)");
            }
            Err(e) => {
                log::error!("Compositor::new: {e}");
                self.window = Some(window);
                self.compositor = None;
                self.fatal_error = Some(format!(
                    "Fallo al inicializar aceleración gráfica (wgpu):\n\n{e}\n\nRecomendaciones para BlueStacks 5:\n- Ve a Ajustes -> Gráficos.\n- Cambia el Renderizador de interfaz (OpenGL / Software).\n- Alterna el Renderizador de gráficos entre OpenGL y DirectX."
                ));
            }
        }
    }

    /// Compone el fotograma visible y el overlay táctil. No-op si no hay
    /// ventana viva. Ante fallos de GPU, presenta la pantalla de error por software.
    fn redraw(&mut self) -> Option<(f64, f64)> {
        // Solo recurrir a renderizado por software en CPU si wgpu falló al crear el compositor.
        // Si el compositor GPU está vivo, ANativeWindow_lock causaría conflicto con el swapchain.
        if self.compositor.is_none() {
            if let Some(error_msg) = self.fatal_error.clone() {
                self.render_software_error(
                    "[BLAZING RECOMP] ERROR DE INICIALIZACIÓN GPU",
                    &error_msg,
                );
            }
            return None;
        }

        let Some(window) = &self.window else { return None; };
        let (w, h) = window.size();
        if w > 0 && h > 0 && self.touch.size() != (w, h) {
            self.touch.resize(w, h);
        }
        let Some(compositor) = self.compositor.as_mut() else { return None; };
        let (frame, is_native) = if let Some(runner) = &self.runner {
            (runner.frame_rgba(), runner.is_native())
        } else {
            (self.placeholder_rgba.as_slice(), false)
        };
        match compositor.render(frame, (w, h), &self.touch, is_native) {
            Ok(timing) => Some(timing),
            Err(e) => {
                log::warn!("compositor: {e}");
                None
            }
        }
    }

    /// Escribe una línea del informe de rendimiento en blazing-perf.txt y en Logcat.
    fn write_perf_report(&mut self, report: &crate::perf::PerfReport) {
        let line = report.to_line();
        log::info!("PERF: {}", line);
        if self.perf_file.is_none() && !self.perf_write_warned {
            let is_native = self.runner.as_ref().map_or(false, |r| r.is_native());
            let backend = self
                .compositor
                .as_ref()
                .map(|c| c.fps_counter.renderer())
                .unwrap_or("Vulkan");
            self.perf_file = init_perf_file(
                &self.search_dirs,
                backend,
                is_native,
                self.clock_cost_ns,
            );
            if self.perf_file.is_none() {
                self.perf_write_warned = true;
            }
        }
        if let Some(ref mut f) = self.perf_file {
            use std::io::Write;
            if let Err(e) = writeln!(f, "{}", line) {
                if !self.perf_write_warned {
                    log::warn!("Fallo al escribir en blazing-perf.txt: {e}");
                    self.perf_write_warned = true;
                }
            } else {
                let _ = f.flush();
            }
        }
    }

    /// Combina la entrada táctil con la de mandos físicos (operación AND bit a bit activa en bajo)
    /// con arbitraje para erradicar dobles pulsaciones en emuladores como BlueStacks.
    #[inline]
    fn combined_keys(&self) -> u16 {
        let touch_keys = if self.physical_key_mask != 0x03FF {
            // Si hay alguna dirección física presionada, enmascarar direcciones táctiles
            let phys_dpad = !self.physical_key_mask & 0x00F0;
            if phys_dpad != 0 {
                self.touch.key_mask() | 0x00F0
            } else {
                self.touch.key_mask()
            }
        } else {
            self.touch.key_mask()
        };
        touch_keys & self.physical_key_mask
    }

    /// Registra el avance de `count` fotogramas de simulación GBA en la telemetría de FPS.
    fn on_sim_frames(&mut self, count: u32) {
        if let Some(comp) = self.compositor.as_mut() {
            comp.fps_counter.on_frames(count);
        }
    }

    /// Avanza un fotograma del juego si hay un runner activo.
    fn step(&mut self) {
        let keys = self.combined_keys();
        if let Some(runner) = self.runner.as_mut() {
            runner.set_keys(keys);
            runner.step_frame();
        }
    }

    /// Publica la máscara KEYINPUT combinada (táctil + mando físico) en `GameRunner`.
    fn publish_keys(&mut self) {
        let keys = self.combined_keys();
        if let Some(runner) = self.runner.as_mut() {
            runner.set_keys(keys);
        }
    }

    /// Procesa eventos de teclado, mandos físicos y control remoto de Android TV.
    fn on_key(&mut self, event: &KeyEvent) -> bool {
        // Filtrar autorepeat del SO (evita repeticiones espurias en teclados de PC / BlueStacks)
        if event.action() == KeyAction::Down && event.repeat_count() > 0 {
            return true;
        }

        let code = event.key_code();
        let button = match code {
            Keycode::DpadUp => crate::input::Button::Up,
            Keycode::DpadDown => crate::input::Button::Down,
            Keycode::DpadLeft => crate::input::Button::Left,
            Keycode::DpadRight => crate::input::Button::Right,
            Keycode::ButtonA | Keycode::DpadCenter | Keycode::Enter => crate::input::Button::A,
            Keycode::ButtonB | Keycode::Back | Keycode::Escape => crate::input::Button::B,
            Keycode::ButtonL1 => crate::input::Button::L,
            Keycode::ButtonR1 => crate::input::Button::R,
            Keycode::ButtonStart | Keycode::MediaPlayPause => crate::input::Button::Start,
            Keycode::ButtonSelect => crate::input::Button::Select,
            _ => return false,
        };

        let bit = button.key_bit();
        match event.action() {
            KeyAction::Down => {
                let prev = self.physical_key_mask;
                self.physical_key_mask &= !bit; // activo en bajo (0 = presionado)
                if prev != self.physical_key_mask {
                    log::debug!("key down: {button:?} (mask: {:#06x})", self.physical_key_mask);
                    self.publish_keys();
                    true
                } else {
                    false
                }
            }
            KeyAction::Up => {
                let prev = self.physical_key_mask;
                self.physical_key_mask |= bit; // 1 = liberado
                if prev != self.physical_key_mask {
                    log::debug!("key up: {button:?} (mask: {:#06x})", self.physical_key_mask);
                    self.publish_keys();
                    true
                } else {
                    false
                }
            }
            _ => false,
        }
    }

    /// Procesa un evento de movimiento táctil: actualiza el estado del
    /// controlador, la máscara de teclas y dispara háptica en las
    /// transiciones de pulsación. Devuelve `true` si el estado de los botones
    /// cambió y requiere redibujar.
    fn on_motion(&mut self, event: &MotionEvent<'_>) -> bool {
        self.touch.touch_enabled = true;
        if let Some(window) = &self.window {
            let (w, h) = window.size();
            if w > 0 && h > 0 && self.touch.size() != (w, h) {
                self.touch.resize(w, h);
            }
        }
        let action = event.action();
        match action {
            MotionAction::Down | MotionAction::PointerDown => {
                let pointer_id = event.pointer_at_index(event.pointer_index()).pointer_id();
                let p = event.pointer_at_index(event.pointer_index());
                let (x, y) = (p.x(), p.y());
                if let Some(button) = self.touch.touch_down(pointer_id, x, y) {
                    log::debug!("touch: {button:?} pulsado en ({x},{y})");
                    if let Some(h) = &self.haptics {
                        h.vibrate(HAPTIC_MS);
                    }
                }
                self.publish_keys();
                true
            }
            MotionAction::Up | MotionAction::PointerUp => {
                let pointer_id = event.pointer_at_index(event.pointer_index()).pointer_id();
                if let Some(button) = self.touch.touch_up(pointer_id) {
                    log::debug!("touch: {button:?} liberado");
                }
                self.publish_keys();
                true
            }
            MotionAction::Cancel => {
                self.touch.cancel_all();
                self.publish_keys();
                true
            }
            _ => false,
        }
    }
}

static LAST_PANIC: std::sync::Mutex<Option<String>> = std::sync::Mutex::new(None);

#[no_mangle]
pub fn android_main(app: android_activity::AndroidApp) {
    // Logging a Logcat con el tag "BlazingRecomp" (visible en `adb logcat`).
    android_logger::init_once(
        android_logger::Config::default()
            .with_max_level(log::LevelFilter::Info)
            .with_tag("BlazingRecomp"),
    );
    log::info!("Blazing Recomp: android_main (fase overlay táctil y marco SP)");

    // Hook global de pánicos: captura el mensaje y ubicación en lugar de aborto silencioso.
    std::panic::set_hook(Box::new(|info| {
        let msg = if let Some(s) = info.payload().downcast_ref::<&str>() {
            s.to_string()
        } else if let Some(s) = info.payload().downcast_ref::<String>() {
            s.clone()
        } else {
            "Pánico nativo en ejecución".to_string()
        };
        let loc = if let Some(l) = info.location() {
            format!("{}:{}:{}", l.file(), l.line(), l.column())
        } else {
            "ubicación desconocida".to_string()
        };
        let full = format!("CRASH CRÍTICO (PANIC):\n\n{msg}\n\nUbicación: {loc}\n\nRevisa Logcat ('adb logcat -s BlazingRecomp') para el reporte completo.");
        log::error!("{full}");
        if let Ok(mut lock) = LAST_PANIC.lock() {
            *lock = Some(full);
        }
    }));

    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        let mut state = AppState::new();
        state.haptics = Haptics::connect(app.vm_as_ptr(), app.activity_as_ptr());

        // Comprobación y solicitud proactiva de permisos de almacenamiento en el arranque
        let permissions = PermissionsManager::connect(app.vm_as_ptr(), app.activity_as_ptr());
        if let Some(ref p) = permissions {
            p.request_storage_permissions();
            if p.is_television() {
                log::info!("Dispositivo Android TV detectado: desactivando overlay táctil en pantalla");
                state.touch.touch_enabled = false;
            }
        }

        // Búsqueda automática de la ROM oficial y BIOS en Downloads y almacenamiento propio
        let internal_dir = app.internal_data_path();
        let external_dir = app.external_data_path();
        if let Some(ref ed) = external_dir {
            let _ = std::fs::create_dir_all(ed);
        }
        let search_dirs = crate::storage::resolve_search_directories(
            internal_dir.as_deref(),
            external_dir.as_deref(),
        );
        state.search_dirs = search_dirs.clone();
        log::info!("Iniciando escaneo de ROM oficial en directorios: {:?}", search_dirs);
        match crate::storage::scan_and_load_game(&search_dirs) {
            Ok(assets) => {
                log::info!(
                    "ROM oficial encontrada en {:?} (BIOS: {:?}). Inicializando motor a 60 FPS.",
                    assets.rom_path,
                    assets.bios_path
                );
                state.load(assets.rom, assets.bios.as_deref());
            }
            Err(e) => {
                log::warn!("ROM oficial no encontrada ({e}). Generando pantalla instructiva en GPU.");
                let ext_path_display = external_dir
                    .as_ref()
                    .map(|p| p.display().to_string())
                    .unwrap_or_else(|| "Android/data/com.blazingrecomp.game/files/".to_string());
                let lines = [
                    "ROM NO DETECTADA (SHA-256)",
                    "",
                    "Opcion A (Downloads):",
                    "  Coloca 'fe7.gba' o",
                    "  'Fire Emblem (Europe).gba'",
                    "  en la carpeta Download/",
                    "",
                    "Opcion B (Sin permisos):",
                    "  Coloca 'fe7.gba' en:",
                    &ext_path_display,
                ];
                crate::storage::render_guide_screen(
                    &mut state.placeholder_rgba,
                    "BLAZING RECOMP - GUIA",
                    &lines,
                );
            }
        }

        let mut finishing = false;
        let mut resumed = false;
        let mut last_tick = std::time::Instant::now();
        let mut lag = std::time::Duration::ZERO;
        let mut last_scan_attempt = std::time::Instant::now();
        const FRAME_DURATION: std::time::Duration = std::time::Duration::from_nanos(16_666_667); // 60 FPS
        const MAX_ACCUMULATED: std::time::Duration = std::time::Duration::from_millis(50); // Prevenir espiral

        while !finishing {
            // Suspender espera activa cuando la app no está en primer plano.
            // Si no hay runner (esperando permisos/ROM), usar un timeout de 100ms para permitir
            // reescaneo periódico sin bloquear indefinidamente en Android 9 emulado.
            let timeout = if resumed && state.window.is_some() && state.runner.is_some() {
                Some(std::time::Duration::ZERO)
            } else if resumed && state.window.is_some() {
                Some(std::time::Duration::from_millis(100))
            } else {
                None
            };

            app.poll_events(timeout, |event| {
                let PollEvent::Main(main) = event else { return };
                match main {
                    MainEvent::InitWindow { .. } => {
                        log::info!("ciclo de vida: InitWindow");
                        state.create_compositor(
                            app.native_window().map(AndroidWindow::new).map(Arc::new),
                        );
                        state.redraw();
                    }
                    MainEvent::WindowResized { .. } | MainEvent::RedrawNeeded { .. } => {
                        log::info!("ciclo de vida: redibujar ventana");
                        state.redraw();
                    }
                    MainEvent::TerminateWindow { .. } => {
                        log::info!("ciclo de vida: TerminateWindow");
                        state.remove_compositor();
                    }
                    MainEvent::Resume { .. } => {
                        log::info!("ciclo de vida: Resume");
                        resumed = true;
                        last_tick = std::time::Instant::now();
                        lag = std::time::Duration::ZERO;
                        if state.runner.is_none() {
                            log::info!("sin GameRunner cargado: reintentando escaneo de ROM tras Resume");
                            if let Ok(assets) = crate::storage::scan_and_load_game(&search_dirs) {
                                log::info!(
                                    "ROM oficial encontrada en {:?} tras Resume. Inicializando motor a 60 FPS.",
                                    assets.rom_path
                                );
                                state.load(assets.rom, assets.bios.as_deref());
                            }
                        }
                    }
                    MainEvent::Pause => {
                        log::info!("ciclo de vida: Pause");
                        resumed = false;
                        state.suspend();
                    }
                    MainEvent::Destroy => {
                        log::info!("ciclo de vida: Destroy");
                        finishing = true;
                    }
                    _ => {}
                }
            });

            let mut needs_redraw = false;
            if let Ok(mut input) = app.input_events_iter() {
                while input.next(|event| match event {
                    InputEvent::MotionEvent(m) => {
                        if state.on_motion(m) {
                            needs_redraw = true;
                        }
                        InputStatus::Handled
                    }
                    InputEvent::KeyEvent(k) => {
                        if state.on_key(k) {
                            needs_redraw = true;
                        }
                        InputStatus::Handled
                    }
                    _ => InputStatus::Unhandled,
                }) {}
            }

            // Si aún no se ha cargado el juego (ej. usuario concedió permisos en Android 9 emulado sin onResume),
            // reintentar escaneo al tocar la pantalla o cada 1 segundo automáticamente.
            if resumed && state.runner.is_none() {
                if needs_redraw || last_scan_attempt.elapsed() >= std::time::Duration::from_millis(1000) {
                    last_scan_attempt = std::time::Instant::now();
                    if let Ok(assets) = crate::storage::scan_and_load_game(&search_dirs) {
                        log::info!(
                            "ROM oficial detectada en {:?}. Inicializando motor.",
                            assets.rom_path
                        );
                        state.load(assets.rom, assets.bios.as_deref());
                        last_tick = std::time::Instant::now();
                        lag = std::time::Duration::ZERO;
                        needs_redraw = true;
                    }
                }
            }

            if resumed && state.window.is_some() {
                let now = std::time::Instant::now();
                let mut delta = now.duration_since(last_tick);
                last_tick = now;
                if delta > MAX_ACCUMULATED {
                    delta = MAX_ACCUMULATED;
                }
                lag += delta;

                if state.runner.is_some() {
                    let mut steps = 0;
                    let mut last_step_timing = crate::runner::StepTiming::default();
                    while lag >= FRAME_DURATION && steps < 2 {
                        state.step();
                        if let Some(runner) = &state.runner {
                            last_step_timing = runner.last_timing();
                        }
                        lag -= FRAME_DURATION;
                        steps += 1;
                    }
                    if lag >= FRAME_DURATION {
                        lag = std::time::Duration::ZERO;
                    }
                    if steps > 0 {
                        state.on_sim_frames(steps);
                    }
                    let gpu_timing = if steps > 0 || needs_redraw {
                        state.redraw()
                    } else {
                        None
                    };

                    if steps > 0 {
                        let (dibujo_ms, espera_ms) = gpu_timing.unwrap_or((0.0, 0.0));
                        for _ in 0..steps {
                            state.perf.record_frame(crate::perf::FrameTiming {
                                ejec_ms: last_step_timing.ejec_ms,
                                color_ms: last_step_timing.color_ms,
                                dibujo_ms: dibujo_ms / (steps as f64),
                                espera_ms: espera_ms / (steps as f64),
                                nativos: last_step_timing.native_blocks,
                                interprete: last_step_timing.interpreter_steps,
                            });
                        }
                    } else if !needs_redraw {
                        state.perf.record_empty_turn();
                    }

                    let (win_w, win_h) = state.window.as_ref().map(|w| w.size()).unwrap_or((0, 0));
                    if let Some(report) = state.perf.tick_at(now, (win_w, win_h)) {
                        state.write_perf_report(&report);
                    }
                } else {
                    state.perf.record_empty_turn();
                    if needs_redraw {
                        state.redraw();
                    }
                }
            } else if needs_redraw {
                state.redraw();
            }
        }
    }));

    if result.is_err() {
        let panic_msg = LAST_PANIC
            .lock()
            .ok()
            .and_then(|mut lock| lock.take())
            .unwrap_or_else(|| "Error fatal desconocido en el hilo de la aplicación".to_string());
        log::error!("android_main capturó pánico no manejado: {panic_msg}");

        if let Some(win) = app.native_window() {
            if let Ok(renderer) = crate::error_screen::ErrorScreenRenderer::new() {
                if let Ok(mut guard) = win.lock(None) {
                    let stride = guard.stride();
                    let height = guard.height();
                    let width = guard.width();
                    let bpp = guard.format().bytes_per_pixel().unwrap_or(4);
                    if let Some(bytes) = guard.bytes() {
                        let slice: &mut [u8] = unsafe {
                            std::slice::from_raw_parts_mut(bytes.as_mut_ptr().cast(), bytes.len())
                        };
                        renderer.render(
                            slice,
                            width,
                            height,
                            stride,
                            bpp,
                            "[BLAZING RECOMP] CRASH CRÍTICO",
                            &panic_msg,
                        );
                    }
                }
            }

            // Mantener ventana abierta para lectura del diagnóstico hasta que Android la destruya
            let mut done = false;
            while !done {
                app.poll_events(None, |event| {
                    if let PollEvent::Main(MainEvent::Destroy) = event {
                        done = true;
                    }
                });
            }
        }
    }

    log::info!("Blazing Recomp: android_main finalizado");
}

