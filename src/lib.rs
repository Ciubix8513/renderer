//!
//! # Lunar engine
//! A small silly engine for fun :3
//!
//!
//! # Project setup
//! Setting up a project is really simple. The application is split into 3 states:
//! 1. Initialization
//! 2. Main loop
//! 3. Disposal
//!
//! First define the state of the app
//!
//! ```
//! struct MyState;
//! ```
//! The state can contain any data that needs to be persistent between frames, for example an
//! `AssetStore` or `World`
//!
//! Define the application functions, all of them have identical signature:
//! ```
//! # struct MyState;
//! fn initialize(state: &mut MyState) {}
//! fn run(state: &mut MyState) {}
//! fn close(state: &mut MyState) {}
//! ```
//! Then create an instance of that state and start the loop of the program
//! ```no_run
//! # #[derive(Default)]
//! # struct MyState;
//! # fn initialize(state: &mut MyState) {}
//! # fn run(state: &mut MyState) {}
//! # fn close(state: &mut MyState) {}
//! fn main() {
//!     let state = lunar_engine::State::<MyState>::default();
//!     state.run(initialize, run, close);
//! }
//! ```
//!

#![allow(
    clippy::needless_doctest_main,
    clippy::cast_possible_truncation,
    clippy::cast_precision_loss,
    clippy::missing_panics_doc
)]
use std::{
    cell::OnceCell,
    sync::{OnceLock, RwLock},
};

use chrono::DateTime;
use wgpu::SurfaceConfiguration;
use winit::{
    application::ApplicationHandler,
    dpi::{PhysicalPosition, PhysicalSize},
    event,
    window::CursorGrabMode,
};

#[cfg(target_arch = "wasm32")]
use wrappers::WgpuWrapper;

pub mod asset_managment;
pub mod assets;
pub mod components;
pub mod ecs;
mod grimoire;
mod helpers;
pub mod import;
pub mod input;
mod logging;
pub mod math;
pub mod structures;
pub mod system;
#[cfg(test)]
mod test_utils;
pub mod windowing;
#[cfg(target_arch = "wasm32")]
mod wrappers;

#[cfg(target_arch = "wasm32")]
pub static DEVICE: OnceLock<wrappers::WgpuWrapper<wgpu::Device>> = OnceLock::new();
#[cfg(target_arch = "wasm32")]
pub static QUEUE: OnceLock<wrappers::WgpuWrapper<wgpu::Queue>> = OnceLock::new();

#[cfg(not(target_arch = "wasm32"))]
pub static DEVICE: OnceLock<wgpu::Device> = OnceLock::new();
#[cfg(not(target_arch = "wasm32"))]
pub static QUEUE: OnceLock<wgpu::Queue> = OnceLock::new();
pub static FORMAT: OnceLock<wgpu::TextureFormat> = OnceLock::new();

#[cfg(target_arch = "wasm32")]
pub static STAGING_BELT: OnceLock<RwLock<wrappers::WgpuWrapper<wgpu::util::StagingBelt>>> =
    OnceLock::new();
#[cfg(not(target_arch = "wasm32"))]
pub static STAGING_BELT: OnceLock<RwLock<wgpu::util::StagingBelt>> = OnceLock::new();
pub static RESOLUTION: RwLock<PhysicalSize<u32>> = RwLock::new(PhysicalSize {
    width: 0,
    height: 0,
});
//TODO find a better way than just staticing it
static WINDOW: OnceLock<winit::window::Window> = OnceLock::new();

#[cfg(target_arch = "wasm32")]
static SURFACE: OnceLock<RwLock<wrappers::WgpuWrapper<wgpu::Surface>>> = OnceLock::new();
#[cfg(target_arch = "wasm32")]
static DEPTH: OnceLock<RwLock<wrappers::WgpuWrapper<wgpu::Texture>>> = OnceLock::new();

#[cfg(not(target_arch = "wasm32"))]
static SURFACE: OnceLock<RwLock<wgpu::Surface>> = OnceLock::new();
#[cfg(not(target_arch = "wasm32"))]
static DEPTH: OnceLock<RwLock<wgpu::Texture>> = OnceLock::new();

static QUIT: OnceLock<bool> = OnceLock::new();
static DELTA_TIME: RwLock<f32> = RwLock::new(0.01);

///Defines behaviour of the cursor inside the window
#[derive(Clone, Copy)]
pub enum CursorState {
    //Cursor is locked to the window
    Locked,
    //Cursor is free
    Free,
}

struct CursorStateInternal {
    grab_mode: CursorState,
    lock_failed: bool,
    visible: bool,
    modified: bool,
}
static CURSOR_STATE: RwLock<CursorStateInternal> = RwLock::new(CursorStateInternal {
    grab_mode: CursorState::Free,
    lock_failed: false,
    visible: true,
    modified: false,
});

fn reset_cursor() {
    let window = WINDOW.get().unwrap();

    let pos = window.inner_size();
    if let Err(e) = window.set_cursor_position(PhysicalPosition {
        x: pos.width / 2,
        y: pos.height / 2,
    }) {
        log::error!("Failed to move cursor {e}");
    }
}

fn process_cursor() {
    let mut state = CURSOR_STATE.write().unwrap();

    if matches!(state.grab_mode, CursorState::Locked) && state.lock_failed {
        reset_cursor();
    }

    if !state.modified {
        return;
    }
    state.modified = false;
    let window = WINDOW.get().unwrap();

    window.set_cursor_visible(state.visible);

    let g_mode = state.grab_mode;
    let res = window.set_cursor_grab(match g_mode {
        CursorState::Locked => CursorGrabMode::Locked,
        CursorState::Free => CursorGrabMode::None,
    });
    if let Err(e) = res {
        match e {
            winit::error::ExternalError::NotSupported(_) => {
                //Once a lock has failed, it can never unfail, so no need to reset this
                //afterwards :3
                //
                //This can only unfail if the user changes platform, buuuut, i literally don't
                //think there's a way that could happen
                state.lock_failed = true;
                drop(state);

                log::warn!("Failed to lock cursor, doing manually");
                if let Err(e) = window.set_cursor_grab(CursorGrabMode::Confined) {
                    log::error!("Cursor is fucked :3 {e}");
                }
                reset_cursor();
            }

            winit::error::ExternalError::Ignored => {
                log::warn!("Cursor state change ignored");
            }
            winit::error::ExternalError::Os(e) => log::error!("Cursor state change error: {e}"),
        }
    }
}

//Exits the application and closes the window
pub fn quit() {
    QUIT.set(true).unwrap();
}

///Returns time between frames in seconds
pub fn delta_time() -> f32 {
    *DELTA_TIME.read().unwrap()
}

//Sets the cursor grab mode
// pub fn set_cursor_grab_mode(mode: CursorState) {
//     let mut state = CURSOR_STATE.write().unwrap();
//     state.grab_mode = mode;
//     state.modified = true;
// }

//Sets the cursor grab mode
// pub fn set_cursor_visible(mode: bool) {
//     let mut state = CURSOR_STATE.write().unwrap();
//     state.visible = mode;
//     state.modified = true;
// }

///Contains main state of the app
#[allow(clippy::type_complexity)]
pub struct State<T> {
    first_resume: bool,
    surface_config: OnceCell<SurfaceConfiguration>,
    contents: T,
    closed: bool,
    frame_start: Option<DateTime<chrono::Local>>,
    init: Option<Box<dyn FnOnce(&mut T)>>,
    run: Option<Box<dyn Fn(&mut T)>>,
    end: Option<Box<dyn FnOnce(&mut T)>>,
}

impl<T: Default> Default for State<T> {
    fn default() -> Self {
        Self {
            first_resume: false,
            surface_config: OnceCell::default(),
            contents: Default::default(),
            closed: Default::default(),
            frame_start: Default::default(),
            init: None,
            run: None,
            end: None,
        }
    }
}

impl<T: 'static> State<T> {
    ///Creates a new state with the given custom state
    pub fn new(contents: T) -> Self {
        Self {
            first_resume: false,
            surface_config: OnceCell::new(),
            contents,
            closed: false,
            frame_start: None,
            init: None,
            run: None,
            end: None,
        }
    }

    /// Starts the application with the 3 provided functions:
    /// 1. Initialization function for setting up assets, scene(s), etc.
    /// 2. Game loop
    /// 3. Disposal function
    //TODO Potentially ask for a window
    #[allow(clippy::missing_panics_doc)]
    pub fn run<F, F1, F2>(mut self, init: F, run: F1, end: F2)
    where
        F: FnOnce(&mut T) + 'static,
        F1: Fn(&mut T) + Copy + 'static,
        F2: FnOnce(&mut T) + Copy + 'static,
    {
        self.init = Some(Box::new(init));
        self.run = Some(Box::new(run));
        self.end = Some(Box::new(end));

        #[cfg(target_arch = "wasm32")]
        {
            std::panic::set_hook(Box::new(|e| {
                log::error!("{e}");
            }));
        }

        //Initialize logging first
        logging::initialize_logging();

        let event_loop = winit::event_loop::EventLoop::new().expect("Failed to create event loop");
        log::debug!("Created event loop");

        #[cfg(not(target_arch = "wasm32"))]
        {
            event_loop
                .run_app(&mut self)
                .expect("Failed to start event loop");
        }
        #[cfg(target_arch = "wasm32")]
        {
            use winit::platform::web::EventLoopExtWebSys;
            event_loop.spawn_app(self);
        }
    }
}

impl<T> ApplicationHandler for State<T> {
    fn resumed(&mut self, event_loop: &winit::event_loop::ActiveEventLoop) {
        if self.first_resume {
            return;
        }

        #[cfg(not(target_arch = "wasm32"))]
        let attributes = winit::window::Window::default_attributes();
        let window;
        #[cfg(target_arch = "wasm32")]
        {
            use wasm_bindgen::JsCast;
            use winit::platform::web::WindowAttributesExtWebSys;

            let mut attributes = winit::window::Window::default_attributes();

            //Acquire a canvas as a base for the window
            let canvas = web_sys::window()
                .unwrap()
                .document()
                .unwrap()
                .get_element_by_id("canvas")
                .expect("Failed to find canvas with id \"canvas\"")
                .dyn_into::<web_sys::HtmlCanvasElement>()
                .unwrap();

            let width = canvas.width();
            let height = canvas.height();

            log::info!("Canvas size = {width} x {height}");

            log::debug!("Found canvas");
            attributes = attributes.with_canvas(Some(canvas));

            window = event_loop
                .create_window(attributes)
                .expect("Failed to create the window");
            //Resize window to the canvas size
            //TODO Find a better solution to this hack
            _ = window.request_inner_size(PhysicalSize::new(width, height));
        }

        #[cfg(not(target_arch = "wasm32"))]
        {
            window = event_loop
                .create_window(attributes)
                .expect("Failed to create the window");
        }

        log::debug!("Created window");

        WINDOW.set(window).unwrap();
        let window = WINDOW.get().unwrap();

        let (surface, config, depth_stencil) = windowing::initialize_gpu(window);

        log::debug!("Inititalized GPU");

        self.surface_config.set(config).unwrap();

        #[cfg(not(target_arch = "wasm32"))]
        {
            SURFACE.set(RwLock::new(surface)).unwrap();
            DEPTH.set(RwLock::new(depth_stencil)).unwrap();
        }
        #[cfg(target_arch = "wasm32")]
        {
            SURFACE.set(RwLock::new(WgpuWrapper::new(surface))).unwrap();
            DEPTH
                .set(RwLock::new(WgpuWrapper::new(depth_stencil)))
                .unwrap();
        }

        self.init.take().unwrap()(&mut self.contents);

        event_loop.set_control_flow(winit::event_loop::ControlFlow::Wait);
    }

    fn window_event(
        &mut self,
        event_loop: &winit::event_loop::ActiveEventLoop,
        _: winit::window::WindowId,
        event: event::WindowEvent,
    ) {
        match event {
            event::WindowEvent::Resized(size) => {
                RESOLUTION.write().unwrap().width = size.width;
                RESOLUTION.write().unwrap().height = size.height;
                self.surface_config.get_mut().unwrap().width = size.width;
                self.surface_config.get_mut().unwrap().height = size.height;
                let device = DEVICE.get().unwrap();

                SURFACE
                    .get()
                    .unwrap()
                    .write()
                    .unwrap()
                    .configure(device, self.surface_config.get().unwrap());
                let desc = windowing::get_depth_descriptor(size.width, size.height);

                #[cfg(target_arch = "wasm32")]
                {
                    **DEPTH.get().unwrap().write().unwrap() = device.create_texture(&desc);
                }

                #[cfg(not(target_arch = "wasm32"))]
                {
                    *DEPTH.get().unwrap().write().unwrap() = device.create_texture(&desc);
                }

                // let bpr = helpers::calculate_bpr(size.width, *FORMAT.get().unwrap());
                // self.screenshot_buffer = device.create_buffer(&wgpu::BufferDescriptor {
                //     label: Some("Screenshot buffer"),
                //     size: bpr * size.height as u64,
                //     usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
                //     mapped_at_creation: false,
                // });
            }
            event::WindowEvent::CloseRequested => {
                event_loop.exit();
                self.closed = true;
            }
            event::WindowEvent::RedrawRequested => {
                //Frame time includes the wait between frames
                if let Some(start) = self.frame_start {
                    let finish = chrono::Local::now();

                    let delta =
                        (finish - start).abs().num_microseconds().unwrap() as f32 / 1_000_000.0;

                    *DELTA_TIME.write().unwrap() = delta;
                }
                self.frame_start = Some(chrono::Local::now());

                process_cursor();

                if QUIT.get().is_some() {
                    event_loop.exit();
                    self.closed = true;
                }
                if self.closed {
                    //This should be fine but needs further testing
                    self.end.take().unwrap()(&mut self.contents);

                    return;
                }
                self.run.as_ref().unwrap()(&mut self.contents);
                input::update();

                WINDOW.get().unwrap().request_redraw();
            }
            event::WindowEvent::KeyboardInput {
                device_id: _,
                event,
                is_synthetic: _,
            } => {
                let state = match event.state {
                    event::ElementState::Pressed => input::KeyState::Down,
                    event::ElementState::Released => input::KeyState::Up,
                };
                let keycode = if let winit::keyboard::PhysicalKey::Code(code) = event.physical_key {
                    Some(code)
                } else {
                    None
                };
                if keycode.is_none() {
                    return;
                }
                input::set_key(keycode.unwrap(), state);
            }
            event::WindowEvent::MouseInput {
                device_id: _,
                state,
                button,
            } => match state {
                event::ElementState::Pressed => {
                    input::set_mouse_button(button, input::KeyState::Down);
                }
                event::ElementState::Released => {
                    input::set_mouse_button(button, input::KeyState::Up);
                }
            },

            event::WindowEvent::CursorMoved {
                device_id: _,
                position,
            } => {
                input::set_cursor_position(math::vec2::Vec2 {
                    x: position.x as f32,
                    y: position.y as f32,
                });
            }
            _ => {}
        }
    }
}
