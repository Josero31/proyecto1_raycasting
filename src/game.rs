use crate::audio::AudioManager;
use crate::fonts::draw_text_small;
use crate::level::{get_level, Level};
use crate::raycaster::{render_scene, DepthBuffer};
use crate::sprites::{Sprite, SpriteKind};
use rand::Rng;
use winit::event::VirtualKeyCode;

#[derive(Copy, Clone, PartialEq, Eq)]
enum Mode {
    Menu,
    Playing,
    Win,
}

pub struct Player {
    pub x: f32,
    pub y: f32,
    pub dir_x: f32,
    pub dir_y: f32,
    pub plane_x: f32,
    pub plane_y: f32,
    pub move_speed: f32,
    pub rot_speed: f32,
}

pub struct Game {
    mode: Mode,
    pub level_index: usize,
    pub level: Level,
    pub player: Player,
    pub pressed: [bool; 256],
    pub fps: f32,
    fps_acc: f32,
    fps_count: u32,
    pub audio: AudioManager,
    pub sprites: Vec<Sprite>,
    pub pellets_remaining: usize,
    pub depth: DepthBuffer,
    mouse_sensitivity: f32,
}

impl Game {
    pub fn new(width: i32, _height: i32) -> anyhow::Result<Self> {
        let level_index = 0;
        let level = get_level(level_index);
        let (px, py) = level.spawn;

        let player = Player {
            x: px as f32 + 0.5,
            y: py as f32 + 0.5,
            dir_x: -1.0,
            dir_y: 0.0,
            plane_x: 0.0,
            plane_y: 0.66,
            move_speed: 3.0,
            rot_speed: 2.0,
        };

        let audio = AudioManager::new();
        let sprites = Self::build_sprites_for_level(&level);
        let pellets_remaining = sprites.iter().filter(|s| s.kind == SpriteKind::Pellet).count();

        Ok(Self {
            mode: Mode::Menu,
            level_index,
            level,
            player,
            pressed: [false; 256],
            fps: 0.0,
            fps_acc: 0.0,
            fps_count: 0,
            audio,
            sprites,
            pellets_remaining,
            depth: DepthBuffer::new(width as usize),
            mouse_sensitivity: 0.0035,
        })
    }

    fn build_sprites_for_level(level: &Level) -> Vec<Sprite> {
        let mut sprites = Vec::new();
        for y in 0..level.h {
            for x in 0..level.w {
                if level.map[(y * level.w + x) as usize] == 0 {
                    if (x, y) == level.spawn {
                        continue;
                    }
                    if (x + y) % 2 == 0 {
                        sprites.push(Sprite::new(x as f32 + 0.5, y as f32 + 0.5, SpriteKind::Pellet));
                    }
                }
            }
        }
        let mut rng = rand::thread_rng();
        for _ in 0..level.ghost_count {
            for _tries in 0..200 {
                let gx = rng.gen_range(1..(level.w - 1));
                let gy = rng.gen_range(1..(level.h - 1));
                if level.map[(gy * level.w + gx) as usize] == 0 {
                    sprites.push(Sprite::new(gx as f32 + 0.5, gy as f32 + 0.5, SpriteKind::Ghost));
                    break;
                }
            }
        }
        sprites
    }

    pub fn on_key(&mut self, key: VirtualKeyCode, pressed: bool) {
        let idx = key as usize;
        if idx < self.pressed.len() {
            self.pressed[idx] = pressed;
        }

        match self.mode {
            Mode::Menu => {
                if pressed {
                    match key {
                        VirtualKeyCode::Key1 => self.start_level(0),
                        VirtualKeyCode::Key2 => self.start_level(1),
                        VirtualKeyCode::Key3 => self.start_level(2),
                        _ => {}
                    }
                }
            }
            Mode::Win => {
                if pressed && key == VirtualKeyCode::Return {
                    self.mode = Mode::Menu;
                }
            }
            Mode::Playing => {}
        }
    }

    pub fn on_mouse_delta(&mut self, dx: f32) {
        if self.mode != Mode::Playing {
            return;
        }
        let angle = -dx * self.mouse_sensitivity;
        self.rotate(angle);
    }

    fn start_level(&mut self, index: usize) {
        self.level_index = index;
        self.level = get_level(index);
        let (px, py) = self.level.spawn;
        self.player.x = px as f32 + 0.5;
        self.player.y = py as f32 + 0.5;
        self.player.dir_x = -1.0;
        self.player.dir_y = 0.0;
        self.player.plane_x = 0.0;
        self.player.plane_y = 0.66;
        self.sprites = Self::build_sprites_for_level(&self.level);
        self.pellets_remaining = self.sprites.iter().filter(|s| s.kind == SpriteKind::Pellet).count();
        self.mode = Mode::Playing;
        self.audio.play_music_loop("assets/music/theme.ogg");
    }

    pub fn update(&mut self, dt: f32) {
        self.fps_count += 1;
        self.fps_acc += dt;
        if self.fps_acc >= 1.0 {
            self.fps = self.fps_count as f32 / self.fps_acc;
            self.fps_acc = 0.0;
            self.fps_count = 0;
        }

        match self.mode {
            Mode::Menu => {}
            Mode::Win => {}
            Mode::Playing => {
                self.handle_input(dt);
                self.update_sprites(dt);
                self.check_collisions_and_pickups();
                if self.pellets_remaining == 0 {
                    self.mode = Mode::Win;
                    self.audio.play_sfx("assets/sfx/win.wav");
                }
            }
        }
    }

    fn handle_input(&mut self, dt: f32) {
        let w_down = self.is_down(VirtualKeyCode::W);
        let s_down = self.is_down(VirtualKeyCode::S);
        let q_down = self.is_down(VirtualKeyCode::Q) || self.is_down(VirtualKeyCode::Left);
        let e_down = self.is_down(VirtualKeyCode::E) || self.is_down(VirtualKeyCode::Right);

        let (dir_x, dir_y, move_speed, rot_speed) =
            (self.player.dir_x, self.player.dir_y, self.player.move_speed, self.player.rot_speed);

        let mut move_x = 0.0;
        let mut move_y = 0.0;

        if w_down {
            move_x += dir_x * move_speed * dt;
            move_y += dir_y * move_speed * dt;
        }
        if s_down {
            move_x -= dir_x * move_speed * dt;
            move_y -= dir_y * move_speed * dt;
        }

        let mut rot = 0.0;
        if q_down {
            rot += rot_speed * dt;
        }
        if e_down {
            rot -= rot_speed * dt;
        }
        if rot.abs() > 0.0 {
            self.rotate(rot);
        }

        self.try_move(move_x, move_y);
    }

    fn rotate(&mut self, angle: f32) {
        let p = &mut self.player;
        let old_dir_x = p.dir_x;
        p.dir_x = p.dir_x * angle.cos() - p.dir_y * angle.sin();
        p.dir_y = old_dir_x * angle.sin() + p.dir_y * angle.cos();

        let old_plane_x = p.plane_x;
        p.plane_x = p.plane_x * angle.cos() - p.plane_y * angle.sin();
        p.plane_y = old_plane_x * angle.sin() + p.plane_y * angle.cos();
    }

    fn try_move(&mut self, dx: f32, dy: f32) {
        let new_x = self.player.x + dx;
        let new_y = self.player.y + dy;

        if !self.is_wall(new_x, self.player.y) {
            self.player.x = new_x;
        }
        if !self.is_wall(self.player.x, new_y) {
            self.player.y = new_y;
        }
    }

    fn is_wall(&self, x: f32, y: f32) -> bool {
        if x < 0.0 || y < 0.0 {
            return true;
        }
        let xi = x as i32;
        let yi = y as i32;
        if xi < 0 || yi < 0 || xi >= self.level.w || yi >= self.level.h {
            return true;
        }
        self.level.tile(xi, yi) > 0
    }

    fn update_sprites(&mut self, dt: f32) {
        let level = &self.level;
        let sprites = &mut self.sprites;
        let mut rng = rand::thread_rng();

        for s in sprites.iter_mut() {
            match s.kind {
                SpriteKind::Pellet => {
                    s.anim_time += dt;
                    if s.anim_time > 0.5 {
                        s.anim_time = 0.0;
                        s.anim_frame = (s.anim_frame + 1) % 2;
                    }
                }
                SpriteKind::Ghost => {
                    s.anim_time += dt;
                    if s.anim_time > 0.3 {
                        s.anim_time = 0.0;
                        s.anim_frame = (s.anim_frame + 1) % 2;
                    }

                    let dir_to_player_x = self.player.x - s.x;
                    let dir_to_player_y = self.player.y - s.y;
                    let mut vx = dir_to_player_x * 0.5 + rng.gen_range(-0.5..0.5);
                    let mut vy = dir_to_player_y * 0.5 + rng.gen_range(-0.5..0.5);
                    let len = (vx * vx + vy * vy).sqrt().max(1e-4);
                    vx /= len;
                    vy /= len;
                    let speed = 1.2;
                    let nx = s.x + vx * speed * dt;
                    let ny = s.y + vy * speed * dt;

                    if !is_wall_level(level, nx, s.y) {
                        s.x = nx;
                    }
                    if !is_wall_level(level, s.x, ny) {
                        s.y = ny;
                    }
                }
            }
        }
    }

    fn check_collisions_and_pickups(&mut self) {
        // Reducimos el radio de recolección para coincidir con pellets más pequeños
        let pickup_r = 0.18f32; // antes 0.25
        let pickup_r2 = pickup_r * pickup_r;

        let mut collected_indices = Vec::new();
        for (i, s) in self.sprites.iter().enumerate() {
            if s.kind == SpriteKind::Pellet {
                let dx = self.player.x - s.x;
                let dy = self.player.y - s.y;
                let dist2 = dx * dx + dy * dy;
                if dist2 < pickup_r2 {
                    collected_indices.push(i);
                }
            }
        }
        if !collected_indices.is_empty() {
            collected_indices.sort_unstable();
            collected_indices.drain(..).rev().for_each(|i| {
                self.sprites.remove(i);
            });
            let collected = collected_indices.len();
            if collected > 0 {
                if self.pellets_remaining >= collected {
                    self.pellets_remaining -= collected;
                } else {
                    self.pellets_remaining = 0;
                }
                self.audio.play_sfx("assets/sfx/pellet.wav");
            }
        }
    }

    fn is_down(&self, key: VirtualKeyCode) -> bool {
        self.pressed[key as usize]
    }

    pub fn render(&mut self, frame: &mut [u8], w: i32, h: i32) {
        match self.mode {
            Mode::Menu => self.render_menu(frame, w, h),
            Mode::Playing => self.render_game(frame, w, h),
            Mode::Win => self.render_win(frame, w, h),
        }
    }

    fn render_menu(&mut self, frame: &mut [u8], w: i32, h: i32) {
        fill(frame, w, h, 0x10, 0x10, 0x18);
        draw_text_small(frame, w, h, 16, 16, "PACMAN 3D - Raycaster", [255, 230, 0, 255]);
        draw_text_small(frame, w, h, 16, 40, "Selecciona un nivel:", [200, 200, 200, 255]);
        draw_text_small(frame, w, h, 16, 60, "[1] Nivel 1", [180, 220, 255, 255]);
        draw_text_small(frame, w, h, 16, 75, "[2] Nivel 2", [180, 220, 255, 255]);
        draw_text_small(frame, w, h, 16, 90, "[3] Nivel 3", [180, 220, 255, 255]);
        draw_text_small(frame, w, h, 16, 120, "Controles: W/S mover, Q/E o Flechas rotar, Mouse rota", [180, 180, 180, 255]);
    }

    fn render_win(&mut self, frame: &mut [u8], w: i32, h: i32) {
        fill(frame, w, h, 0, 40, 0);
        draw_text_small(frame, w, h, 16, 16, "¡Nivel completado!", [255, 255, 255, 255]);
        draw_text_small(frame, w, h, 16, 40, "Presiona Enter para volver al menu", [200, 200, 200, 255]);
    }

    fn render_game(&mut self, frame: &mut [u8], w: i32, h: i32) {
        render_scene(frame, w, h, &self.level, &self.player, &self.sprites, &mut self.depth);

        let fps_txt = format!("FPS: {:.0}", self.fps);
        draw_text_small(frame, w, h, 6, 6, &fps_txt, [255, 255, 255, 255]);

        let pellets_txt = format!("Pellets: {}", self.pellets_remaining);
        draw_text_small(frame, w, h, 6, 20, &pellets_txt, [255, 255, 0, 255]);

        self.render_minimap(frame, w, h);
    }

    fn render_minimap(&self, frame: &mut [u8], w: i32, h: i32) {
        let scale = 4;
        let pad = 6;
        let map_w = self.level.w as i32 * scale;
        let map_h = self.level.h as i32 * scale;

        let origin_x = w - map_w - pad;
        let origin_y = pad;

        rect_fill(frame, w, h, origin_x - 2, origin_y - 2, map_w + 4, map_h + 4, [0, 0, 0, 180]);

        // Mapa
        for y in 0..self.level.h {
            for x in 0..self.level.w {
                let tile = self.level.tile(x, y);
                let color = if tile == 0 {
                    [30, 30, 30, 255]
                } else {
                    wall_color(tile)
                };
                rect_fill(
                    frame,
                    w,
                    h,
                    origin_x + x * scale,
                    origin_y + y * scale,
                    scale,
                    scale,
                    color,
                );
            }
        }

        // Pelotas opcional: no las dibujamos para no saturar, pedido fue ver fantasmas

        // Fantasmas en el minimapa (puntos rojos/rosados)
        for s in &self.sprites {
            if s.kind == SpriteKind::Ghost {
                let gx = origin_x as f32 + s.x * scale as f32;
                let gy = origin_y as f32 + s.y * scale as f32;
                rect_fill(frame, w, h, gx as i32 - 1, gy as i32 - 1, 3, 3, [255, 80, 80, 255]);
            }
        }

        // Jugador
        let px = origin_x as f32 + self.player.x * scale as f32;
        let py = origin_y as f32 + self.player.y * scale as f32;
        rect_fill(frame, w, h, px as i32 - 2, py as i32 - 2, 4, 4, [255, 255, 0, 255]);
        let dx = self.player.dir_x * 6.0;
        let dy = self.player.dir_y * 6.0;
        line(frame, w, h, px as i32, py as i32, (px + dx) as i32, (py + dy) as i32, [255, 255, 255, 255]);
    }
}

pub fn wall_color(id: i32) -> [u8; 4] {
    match id % 6 {
        0 => [200, 60, 60, 255],
        1 => [60, 200, 60, 255],
        2 => [60, 60, 200, 255],
        3 => [200, 200, 60, 255],
        4 => [200, 60, 200, 255],
        _ => [60, 200, 200, 255],
    }
}

fn is_wall_level(level: &Level, x: f32, y: f32) -> bool {
    if x < 0.0 || y < 0.0 {
        return true;
    }
    let xi = x as i32;
    let yi = y as i32;
    level.tile(xi, yi) > 0
}

fn fill(frame: &mut [u8], w: i32, h: i32, r: u8, g: u8, b: u8) {
    for y in 0..h {
        for x in 0..w {
            let idx = ((y * w + x) * 4) as usize;
            frame[idx] = r;
            frame[idx + 1] = g;
            frame[idx + 2] = b;
            frame[idx + 3] = 255;
        }
    }
}

fn rect_fill(frame: &mut [u8], w: i32, h: i32, x: i32, y: i32, rw: i32, rh: i32, color: [u8; 4]) {
    for yy in y.max(0)..(y + rh).min(h) {
        for xx in x.max(0)..(x + rw).min(w) {
            let idx = ((yy * w + xx) * 4) as usize;
            frame[idx..idx + 4].copy_from_slice(&color);
        }
    }
}

fn line(frame: &mut [u8], w: i32, h: i32, x0: i32, y0: i32, x1: i32, y1: i32, color: [u8; 4]) {
    let mut x0 = x0;
    let mut y0 = y0;
    let dx = (x1 - x0).abs();
    let sx = if x0 < x1 { 1 } else { -1 };
    let dy = -(y1 - y0).abs();
    let sy = if y0 < y1 { 1 } else { -1 };
    let mut err = dx + dy;
    loop {
        if x0 >= 0 && x0 < w && y0 >= 0 && y0 < h {
            let idx = ((y0 * w + x0) * 4) as usize;
            frame[idx..idx + 4].copy_from_slice(&color);
        }
        if x0 == x1 && y0 == y1 {
            break;
        }
        let e2 = 2 * err;
        if e2 >= dy {
            err += dy;
            x0 += sx;
        }
        if e2 <= dx {
            err += dx;
            y0 += sy;
        }
    }
}