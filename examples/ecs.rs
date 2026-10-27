//! The balls demo from src/main.rs, structured as an entity component system with `hecs`.
//!
//! The schedule is the body of `frame`: adding, removing or reordering a system is a plain code
//! change there, so it hotpatches like any other edit. Changing a component's fields, or what
//! `new` sets up, still needs a restart.

use hecs::World;
use raylib::prelude::*;
use raywasm::Game;

// Components
struct Position(Vector2);
struct Velocity(Vector2);
struct Radius(f32);
struct Tint(Color);

/// The world plus state that isn't per entity
struct Ecs {
    world: World,
    camera: Camera2D,
    time: f32,
    spawned: usize,
}

impl Game for Ecs {
    fn new(rl: &mut RaylibHandle, _thread: &RaylibThread) -> Self {
        let mut ecs = Ecs {
            world: World::new(),
            camera: Camera2D { zoom: 1.0, ..Default::default() },
            time: 0.0,
            spawned: 0,
        };
        let (w, h) = raywasm::logical_size(rl);
        for i in 0..12 {
            let t = i as f32 / 12.0;
            spawn_ball(&mut ecs, Vector2 { x: w * (0.1 + 0.8 * t), y: h * 0.5 }, t * std::f32::consts::TAU);
        }
        ecs
    }

    fn frame(&mut self, rl: &mut RaylibHandle, thread: &RaylibThread) {
        let dt = rl.get_frame_time().min(0.1);
        self.time += dt;
        self.camera.zoom = raywasm::camera_zoom(rl);

        spawn_on_click(self, rl);
        movement(&mut self.world, dt);
        bounce(&mut self.world, raywasm::logical_size(rl));
        render(self, rl, thread);
    }
}

fn spawn_ball(ecs: &mut Ecs, pos: Vector2, angle: f32) {
    let n = ecs.spawned;
    ecs.spawned += 1;
    let speed = 150.0 + n as f32 % 7.0 * 30.0;
    ecs.world.spawn((
        Position(pos),
        Velocity(Vector2 { x: angle.cos() * speed, y: angle.sin() * speed }),
        Radius(12.0 + (n % 5) as f32 * 6.0),
        Tint(Color::color_from_hsv((n as f32 * 37.0) % 360.0, 0.7, 0.95)),
    ));
}

fn spawn_on_click(ecs: &mut Ecs, rl: &RaylibHandle) {
    if rl.is_mouse_button_down(MouseButton::MOUSE_BUTTON_LEFT) {
        let mouse = rl.get_screen_to_world2D(rl.get_mouse_position(), ecs.camera);
        spawn_ball(ecs, mouse, ecs.time * 2.3);
    }
}

fn movement(world: &mut World, dt: f32) {
    for (Position(pos), Velocity(vel)) in world.query_mut::<(&mut Position, &Velocity)>() {
        pos.x += vel.x * dt;
        pos.y += vel.y * dt;
    }
}

fn bounce(world: &mut World, (w, h): (f32, f32)) {
    for (Position(pos), Velocity(vel), Radius(r)) in
        world.query_mut::<(&mut Position, &mut Velocity, &Radius)>()
    {
        let r = *r;
        // Clamping (not just reflecting) keeps balls inside when the window shrinks
        if pos.x < r || pos.x > w - r {
            vel.x = if pos.x < r { vel.x.abs() } else { -vel.x.abs() };
            pos.x = pos.x.clamp(r, (w - r).max(r));
        }
        if pos.y < r || pos.y > h - r {
            vel.y = if pos.y < r { vel.y.abs() } else { -vel.y.abs() };
            pos.y = pos.y.clamp(r, (h - r).max(r));
        }
    }
}

fn render(ecs: &mut Ecs, rl: &mut RaylibHandle, thread: &RaylibThread) {
    let (w, h) = raywasm::logical_size(rl);
    let (fb_w, fb_h) = raywasm::framebuffer_size(rl);
    let scale = raywasm::pixel_ratio(rl);
    let fps = rl.get_fps();
    let mouse = rl.get_screen_to_world2D(rl.get_mouse_position(), ecs.camera);

    let mut d = rl.begin_drawing(thread);
    d.clear_background(Color { r: 18, g: 18, b: 28, a: 255 });
    let mut d = d.begin_mode2D(ecs.camera);

    d.draw_rectangle_lines_ex(Rectangle { x: 0.0, y: 0.0, width: w, height: h }, 4.0, Color::GOLD);
    for (Position(pos), Radius(r), Tint(color)) in ecs.world.query_mut::<(&Position, &Radius, &Tint)>() {
        d.draw_circle_v(*pos, *r, *color);
    }
    d.draw_circle_lines(mouse.x as i32, mouse.y as i32, 20.0, Color::RAYWHITE);
    let info = format!(
        "{w:.0}x{h:.0} css px  |  {fb_w}x{fb_h} framebuffer  |  dpr {scale:.2}  |  {fps} fps  |  {} entities",
        ecs.world.len()
    );
    d.draw_text(&info, 20, 84, 20, Color::LIGHTGRAY);
}

fn main() {
    raywasm::run::<Ecs>("raywasm ecs");
}
