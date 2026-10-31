
use std::num::NonZeroU32;
use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};

use image::{RgbaImage, imageops::FilterType};
use softbuffer::{Context, Surface};
use winit::{
    application::ApplicationHandler,
    dpi::{PhysicalPosition, PhysicalSize},
    event::WindowEvent,
    event_loop::{ActiveEventLoop, ControlFlow, EventLoop, OwnedDisplayHandle},
    window::{Window, WindowAttributes, WindowId},
};

#[cfg(target_os = "windows")]
use winit::platform::windows::EventLoopBuilderExtWindows;

static SPLASH_SCREEN_STATE: AtomicBool = AtomicBool::new(true);

const WIDTH: u32 = 670;
const HEIGHT: u32 = 360;

struct SplashScreen {
    window: Option<Arc<Window>>,
    context: Option<Context<OwnedDisplayHandle>>,
    surface: Option<Surface<OwnedDisplayHandle, Arc<Window>>>,
    pixels: Vec<u32>,
}

impl SplashScreen {
    fn render(&mut self) {
        let (Some(window), Some(surface)) =
            (&self.window, &mut self.surface)
        else {
            return;
        };

        let size = window.inner_size();

        let (Some(width), Some(height)) = (
            NonZeroU32::new(size.width),
            NonZeroU32::new(size.height),
        ) else {
            return;
        };

        if surface.resize(width, height).is_err() {
            return;
        }

        let mut buffer = match surface.buffer_mut() {
            Ok(buffer) => buffer,
            Err(error) => {
                eprintln!("Failed to get splash buffer: {error}");
                return;
            }
        };

        if buffer.len() != self.pixels.len() {
            return;
        }

        buffer.copy_from_slice(&self.pixels);

        if let Err(error) = buffer.present() {
            eprintln!("Failed to present splash screen: {error}");
        }
    }
}

impl ApplicationHandler for SplashScreen {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.window.is_some() {
            return;
        }

        let attributes = WindowAttributes::default()
            .with_title("Luma")
            .with_decorations(false)
            .with_resizable(false)
            .with_inner_size(PhysicalSize::new(WIDTH, HEIGHT));

        let window = Arc::new(
            event_loop
                .create_window(attributes)
                .expect("Failed to create splash screen"),
        );

        if let Some(monitor) = window.current_monitor() {
            let monitor_position = monitor.position();
            let monitor_size = monitor.size();

            let x = monitor_position.x
                + (monitor_size.width as i32 - WIDTH as i32) / 2;

            let y = monitor_position.y
                + (monitor_size.height as i32 - HEIGHT as i32) / 2;

            window.set_outer_position(PhysicalPosition::new(x, y));
        }

        let context = Context::new(event_loop.owned_display_handle())
            .expect("Failed to create softbuffer context");

        let surface = Surface::new(&context, window.clone())
            .expect("Failed to create softbuffer surface");

        self.context = Some(context);
        self.surface = Some(surface);
        self.window = Some(window.clone());

        window.request_redraw();
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        _window_id: WindowId,
        event: WindowEvent,
    ) {
        match event {
            WindowEvent::CloseRequested => {
                SPLASH_SCREEN_STATE.store(false, Ordering::Relaxed);
                event_loop.exit();
            }

            WindowEvent::RedrawRequested => {
                self.render();
            }

            WindowEvent::Resized(_) => {
                if let Some(window) = &self.window {
                    window.request_redraw();
                }
            }

            _ => {}
        }
    }

    fn about_to_wait(&mut self, event_loop: &ActiveEventLoop) {
        if !SPLASH_SCREEN_STATE.load(Ordering::Relaxed) {
            event_loop.exit();
            return;
        }

        event_loop.set_control_flow(ControlFlow::Wait);
    }
}

pub async fn start_splash_screen() {
    SPLASH_SCREEN_STATE.store(true, Ordering::Relaxed);

    // Load image into memory.
    let image: RgbaImage = image::load_from_memory(
        include_bytes!("../resources/splash.png"),
    )
        .expect("Failed to load splash image")
        .to_rgba8();

    // Resize image once.
    let resized = image::imageops::resize(
        &image,
        WIDTH,
        HEIGHT,
        FilterType::Lanczos3,
    );

    // Pre-convert RGBA pixels to Softbuffer's packed RGB format.
    let pixels: Vec<u32> = resized
        .pixels()
        .map(|pixel| {
            let [r, g, b, a] = pixel.0;

            // Composite transparency against black.
            let r = (r as u32 * a as u32) / 255;
            let g = (g as u32 * a as u32) / 255;
            let b = (b as u32 * a as u32) / 255;

            (r << 16) | (g << 8) | b
        })
        .collect();

    // Create event loop.
    let mut builder = EventLoop::builder();

    #[cfg(target_os = "windows")]
    builder.with_any_thread(true);

    let event_loop = builder
        .build()
        .expect("Failed to create event loop");

    let mut app = SplashScreen {
        window: None,
        context: None,
        surface: None,
        pixels,
    };

    event_loop
        .run_app(&mut app)
        .expect("Splash screen event loop failed");
}

pub fn close_splash_screen() {
    SPLASH_SCREEN_STATE.store(false, Ordering::Relaxed);
}
