//! Platform layer: window setup, HiDPI handling, the main loop on web and native, and hotpatching.
//!
//! A game is any type implementing [`Game`]; how it structures its state is up to it.
//! Game logic works in logical pixels (CSS pixels on the web, DPI-scaled pixels on desktop).

use raylib::prelude::*;

pub trait Game: 'static {
    fn new(rl: &mut RaylibHandle, thread: &RaylibThread) -> Self;

    /// Called once per frame. Under `./build.sh hot` this is the hotpatch entry point, so edits
    /// to it (and to anything it calls) take effect at the start of the next frame.
    fn frame(&mut self, rl: &mut RaylibHandle, thread: &RaylibThread);
}

/// Physical pixels per logical pixel (CSS pixels on the web, DPI scale on desktop).
pub fn pixel_ratio(rl: &RaylibHandle) -> f32 {
    #[cfg(target_os = "emscripten")]
    {
        let _ = rl;
        web::device_pixel_ratio()
    }
    #[cfg(not(target_os = "emscripten"))]
    return rl.get_window_scale_dpi().x;
}

/// Zoom a `Camera2D` has to apply to map logical pixels to framebuffer pixels.
pub fn camera_zoom(rl: &RaylibHandle) -> f32 {
    #[cfg(target_os = "emscripten")]
    return pixel_ratio(rl);
    // With FLAG_WINDOW_HIGHDPI raylib already applies the DPI scale to every draw and to the mouse
    #[cfg(not(target_os = "emscripten"))]
    {
        let _ = rl;
        1.0
    }
}

pub fn logical_size(rl: &RaylibHandle) -> (f32, f32) {
    #[cfg(target_os = "emscripten")]
    {
        let s = pixel_ratio(rl);
        (rl.get_screen_width() as f32 / s, rl.get_screen_height() as f32 / s)
    }
    #[cfg(not(target_os = "emscripten"))]
    (rl.get_screen_width() as f32, rl.get_screen_height() as f32)
}

pub fn framebuffer_size(rl: &RaylibHandle) -> (i32, i32) {
    #[cfg(target_os = "emscripten")]
    return (rl.get_screen_width(), rl.get_screen_height());
    #[cfg(not(target_os = "emscripten"))]
    return (rl.get_render_width(), rl.get_render_height());
}

/// Opens the window and runs `G` until it closes (native) or forever (web).
pub fn run<G: Game>(title: &str) {
    #[cfg(target_os = "emscripten")]
    {
        let (w, h) = web::target_canvas_size();
        let (mut rl, thread) = raylib::init().size(w, h).title(title).build();
        let game = G::new(&mut rl, &thread);
        web::run(web::App { rl, thread, game });
    }

    #[cfg(not(target_os = "emscripten"))]
    {
        // vsync rather than a fixed fps cap, like requestAnimationFrame on the web
        let (mut rl, thread) = raylib::init()
            .size(1280, 720)
            .title(title)
            .resizable()
            .highdpi()
            .vsync()
            .build();
        let mut game = G::new(&mut rl, &thread);

        // Under `dx serve --hot-patch` (./build.sh hot), code changes are patched into the running game.
        // `G::frame` is the hot entry point: the jump table is looked up by its address, so a
        // patch takes effect at the start of the next frame. (Wrapping it in a closure would not
        // work, because the closure itself never changes and keeps calling the old `frame`.)
        #[cfg(feature = "hotpatch")]
        dioxus_devtools::connect_subsecond();
        while !rl.window_should_close() {
            #[cfg(feature = "hotpatch")]
            subsecond::HotFn::current(G::frame).call((&mut game, &mut rl, &thread));
            #[cfg(not(feature = "hotpatch"))]
            game.frame(&mut rl, &thread);
        }
    }
}

#[cfg(target_os = "emscripten")]
mod web {
    use raylib::prelude::*;
    use std::ffi::{c_char, c_int, c_void};

    unsafe extern "C" {
        fn emscripten_set_main_loop_arg(
            func: extern "C" fn(*mut c_void),
            arg: *mut c_void,
            fps: c_int,
            simulate_infinite_loop: c_int,
        );
        fn emscripten_get_device_pixel_ratio() -> f64;
        fn emscripten_get_element_css_size(target: *const c_char, w: *mut f64, h: *mut f64) -> c_int;
    }

    pub struct App<G> {
        pub rl: RaylibHandle,
        pub thread: RaylibThread,
        pub game: G,
    }

    pub fn device_pixel_ratio() -> f32 {
        unsafe { emscripten_get_device_pixel_ratio() as f32 }
    }

    pub fn target_canvas_size() -> (i32, i32) {
        let (mut w, mut h) = (0.0, 0.0);
        unsafe { emscripten_get_element_css_size(c"#canvas".as_ptr(), &mut w, &mut h) };
        let dpr = unsafe { emscripten_get_device_pixel_ratio() };
        (((w * dpr).round() as i32).max(1), ((h * dpr).round() as i32).max(1))
    }

    fn fit_canvas_to_page(rl: &mut RaylibHandle) {
        let (w, h) = target_canvas_size();
        if w != rl.get_screen_width() || h != rl.get_screen_height() {
            rl.set_window_size(w, h);
        }
    }

    pub fn run<G: super::Game>(app: App<G>) {
        extern "C" fn tick<G: super::Game>(arg: *mut c_void) {
            let app = unsafe { &mut *(arg as *mut App<G>) };
            fit_canvas_to_page(&mut app.rl);
            app.game.frame(&mut app.rl, &app.thread);
        }
        let app = Box::into_raw(Box::new(app));
        // simulate_infinite_loop = 0: return normally and let main exit; the loop keeps running.
        // With 1, emscripten leaves main by throwing a JS exception, and wasm exception handling
        // runs Rust cleanup code on the way out. With LTO that dropped the RaylibHandle, closing
        // the window before the first frame.
        unsafe { emscripten_set_main_loop_arg(tick::<G>, app as *mut c_void, 0, 0) };
    }
}
