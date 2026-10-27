use raylib::prelude::*;
use raywasm::Game;

struct MyGame {
    camera: Camera2D,
    time: f32,
    x: f32,
}

impl Game for MyGame {
    fn new(_rl: &mut RaylibHandle, _thread: &RaylibThread) -> Self {
        MyGame {
            camera: Camera2D { zoom: 1.0, ..Default::default() },
            time: 0.0,
            x: 0.0,
        }
    }

    fn frame(&mut self, rl: &mut RaylibHandle, thread: &RaylibThread) {
        let (w, h) = raywasm::logical_size(rl);
        let dt = rl.get_frame_time().min(0.1);
        self.time += dt;
        self.camera.zoom = raywasm::camera_zoom(rl);
        let mouse = rl.get_screen_to_world2D(rl.get_mouse_position(), self.camera);

        let fps = rl.get_fps();

        let mut d = rl.begin_drawing(thread);
        d.clear_background(Color { r: 0, g: 0, b: 0, a: 255 });
        let mut d = d.begin_mode2D(self.camera);
        d.draw_text("awdijawoidjoaiwjdowaij", w as i32 / 2, h as i32 / 2, 25, Color::WHITE);
        d.draw_text(&format!("{fps}"), 0, 0, 30, Color::GOLD);
        d.draw_rectangle(self.x as i32, 0, 64, 64, Color::PINK);
        d.draw_circle(mouse.x as i32, mouse.y as i32, 32f32, Color::RED);
    }
}

fn main() {
    raywasm::run::<MyGame>("raywasm");
}
