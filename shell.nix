{ pkgs ? import <nixpkgs> { } }:

let
  # GLFW and miniaudio dlopen these at runtime on native builds
  runtimeLibs = with pkgs; [
    libGL
    libX11 libXrandr libXinerama libXcursor libXi libXext
    alsa-lib libpulseaudio
  ];
in
pkgs.mkShell {
  # bindgenHook sets LIBCLANG_PATH and points bindgen at the native libc headers
  packages = with pkgs; [ emscripten cmake python3 pkg-config rustPlatform.bindgenHook dioxus-cli ];
  buildInputs = runtimeLibs;
  LD_LIBRARY_PATH = pkgs.lib.makeLibraryPath runtimeLibs;
  shellHook = ''
    # the emscripten cache in the nix store is read-only
    export EM_CACHE="$PWD/target/em-cache"
    # raylib-sys runs bindgen against the emscripten headers (overrides bindgenHook's args for wasm)
    export BINDGEN_EXTRA_CLANG_ARGS_wasm32_unknown_emscripten="--sysroot=$EM_CACHE/sysroot"
    # mkShell sets CC=gcc, which the cc/cmake crates would otherwise pick up
    export CC_wasm32_unknown_emscripten=emcc CXX_wasm32_unknown_emscripten=em++ AR_wasm32_unknown_emscripten=emar
  '';
}
