# raywasm

A minimal [raylib](https://www.raylib.com/) game in Rust. It compiles to WebAssembly with Emscripten, where it runs fullscreen in the browser, or to a native desktop binary.

## Build and run

```sh
rustup target add wasm32-unknown-emscripten   # once
nix-shell                                     # emscripten, cmake, libclang, X11/GL libs
./build.sh web --serve                        # builds into dist/web/ and serves it on http://localhost:8080
./build.sh native --run                       # builds into dist/native/ and runs it
./build.sh native --example ecs --run         # the same for examples/ecs.rs (into dist/native-ecs/)
```

Anything after `--` is passed on to `cargo build` (or `dx serve` for `hot`).

Plain `cargo run` also builds and runs the native version, since there is no default target in `.cargo/config.toml`.

The native binary uses GLFW's X11 backend (raylib-sys's default), which also works under XWayland. Outside `nix-shell` it needs `LD_LIBRARY_PATH` to find libGL and the X11 libraries on NixOS.

## Code layout

- `src/lib.rs` is the platform layer: window setup, HiDPI, the web and native main loops, and hotpatching. It runs any type that implements the `Game` trait (`new` and `frame`), and has helpers like `logical_size` and `camera_zoom` for the HiDPI handling below.
- `src/main.rs` is the game: the balls demo as plain structs.
- `examples/ecs.rs` is the same demo structured as an entity component system with [hecs](https://docs.rs/hecs). The schedule is just the body of `frame`, a list of system calls. It's one way to structure a game, not something the template needs. `hecs` is only a dev-dependency, so add it to `[dependencies]` in a game that uses it.

## Hotpatching (native only)

```sh
./build.sh hot                  # = dx serve --hot-patch --platform linux --features hotpatch
./build.sh hot --example ecs
```

This runs the game under the Dioxus CLI (`dx`, from `shell.nix`) with [subsecond](https://docs.rs/subsecond) hotpatching. When you save a file, `dx` rebuilds only the changed code (~300 ms) and patches it into the running game at the start of the next frame. The game state (balls, window) survives.

- The `hotpatch` cargo feature is off by default, so normal and web builds don't include any of this.
- `Game::frame` is the hot entry point. Changing function bodies works, including functions that `frame` calls. In the ECS example that includes adding, removing or reordering systems in `frame`, because the schedule is just code there.
- Changing a struct's layout (adding a field to `Balls`, `Ball` or an ECS component), or code that only runs at startup such as `Game::new`, needs a restart: press `r` in `dx`.
- Keep the `subsecond` and `dioxus-devtools` crates on the same minor version as `dx`. They're both 0.7 right now.
- Disk usage: `dx` builds in its own profile (`target/desktop-dev`, `target/x86_64-unknown-linux-gnu/desktop-dev`, a few hundred MB, built once). Each patch writes a ~150 KB `libraywasm-patch-*.so` to `target/dx/`, and `dx` clears those when it restarts.

## How fullscreen and HiDPI work

- `web/index.html` sets the canvas's CSS size to the whole viewport.
- Every frame, before calling `Game::frame`, the platform layer reads the canvas's CSS size and `devicePixelRatio`. It sets the framebuffer to `css size × dpr` whenever the result changes. This covers browser resizes, zoom, and moving the window between monitors. The image is never stretched and stays sharp on HiDPI screens.
- The game logic works in CSS pixels. A `Camera2D` with `zoom = raywasm::camera_zoom(rl)` (the `dpr` on the web) maps those to framebuffer pixels, and `get_screen_to_world2D` with that camera turns mouse positions back into CSS pixels.
- On the web, `main` returns right after starting the main loop (`emscripten_set_main_loop_arg` with `simulate_infinite_loop = 0`). The alternative, `1`, leaves `main` by throwing a JS exception, and with wasm exception handling Rust's cleanup code runs on the way out. In LTO builds that dropped the `RaylibHandle` and closed the window before the first frame.
- raylib's own `FLAG_WINDOW_RESIZABLE` is not used on the web, because it sizes the canvas to `innerWidth` and ignores the pixel ratio.
- On desktop the window uses `FLAG_WINDOW_HIGHDPI`. raylib then reports the screen size in logical pixels, scales every draw call and the mouse position itself, and renders at full resolution. So the camera zoom stays at 1 there, and the game sees the same logical coordinates as on the web.
- Native uses vsync instead of a fixed fps cap, so it runs at the display refresh rate like `requestAnimationFrame` does in the browser.

## Known issue: vertex precision on some mobile GPUs

On the web, raylib builds for WebGL 1. Its default vertex shader declares `precision mediump float;` (`rlgl.h`). Many mobile GPUs, especially older Mali, Adreno and PowerVR chips, run `mediump` as 16-bit floats. `Camera2D` is applied in the shader (`gl_Position = mvp * vec4(vertexPosition, 1.0)`), so the vertex buffer holds raw world coordinates. At a few thousand world units, vertices start to snap and jitter. Above 65504 they overflow.

Possible future fix: copy raylib's default shader but use `precision highp float;` in the vertex stage. WebGL 1 requires `highp` support in vertex shaders. Load the shader once and wrap each frame in `begin_shader_mode` so every draw call uses it.
