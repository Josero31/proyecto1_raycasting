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
    Paused,
    Win,
    GameOver,
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

    // Vidas y estado
    pub lives: i32,        // 3 vidas por nivel
    invincible_time: f32,  // invulnerabilidad tras perder vida
    time: f32,             // tiempo global (IA)
    death_anim_t: f32,     // animación de game over

    // Contador total de monedas del nivel
    pub total_pellets: usize,
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
        let total_pellets = sprites.iter().filter(|s| s.kind == SpriteKind::Pellet).count();
        let pellets_remaining = total_pellets;

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

            lives: 3,
            invincible_time: 0.0,
            time: 0.0,
            death_anim_t: 0.0,

            total_pellets,
        })
    }

    // Menos monedas: aprox 1 de cada 6 celdas vacías, determinista por coordenadas
    fn build_sprites_for_level(level: &Level) -> Vec<Sprite> {
        let mut sprites = Vec::new();

        for y in 0..level.h {
            for x in 0..level.w {
                if level.map[(y * level.w + x) as usize] == 0 {
                    if (x, y) == level.spawn {
                        continue;
                    }
                    if ((x + y * 3) % 6) == 0 {
                        sprites.push(Sprite::new(x as f32 + 0.5, y as f32 + 0.5, SpriteKind::Pellet));
                    }
                }
            }
        }

        // Garantiza al menos 1 pellet por nivel
        if !sprites.iter().any(|s| s.kind == SpriteKind::Pellet) {
            'outer: for y in 1..level.h - 1 {
                for x in 1..level.w - 1 {
                    if level.map[(y * level.w + x) as usize] == 0 && (x, y) != level.spawn {
                        sprites.push(Sprite::new(x as f32 + 0.5, y as f32 + 0.5, SpriteKind::Pellet));
                        break 'outer;
                    }
                }
            }
        }

        // Fantasmas en posiciones aleatorias válidas
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
            Mode::GameOver => {
                if pressed {
                    match key {
                        VirtualKeyCode::R => {
                            // Reintentar este nivel
                            self.start_level(self.level_index);
                        }
                        VirtualKeyCode::Return => {
                            // Volver al menú
                            self.mode = Mode::Menu;
                        }
                        _ => {}
                    }
                }
            }
            Mode::Paused => {
                if pressed {
                    match key {
                        VirtualKeyCode::P => {
                            // Reanudar
                            self.mode = Mode::Playing;
                        }
                        VirtualKeyCode::Return => {
                            // Volver al menú desde pausa
                            self.mode = Mode::Menu;
                        }
                        _ => {}
                    }
                }
            }
            Mode::Playing => {
                if pressed && key == VirtualKeyCode::P {
                    // Pausa
                    self.mode = Mode::Paused;
                }
            }
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

        // Recalcular contadores de monedas
        self.total_pellets = self.sprites.iter().filter(|s| s.kind == SpriteKind::Pellet).count();
        self.pellets_remaining = self.total_pellets;

        self.mode = Mode::Playing;
        self.lives = 3;             // 3 vidas por nivel
        self.invincible_time = 0.0; // sin invulnerabilidad al inicio
        self.death_anim_t = 0.0;
        self.time = 0.0;

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
            Mode::GameOver => {
                // Animación de Game Over
                self.death_anim_t += dt;
            }
            Mode::Paused => {
                // En pausa no actualizamos lógica ni temporizadores de juego.
            }
            Mode::Playing => {
                self.time += dt;
                if self.invincible_time > 0.0 {
                    self.invincible_time = (self.invincible_time - dt).max(0.0);
                }

                self.handle_input(dt);
                self.update_sprites(dt);
                self.check_collisions_and_pickups();

                // Victoria al recolectar todas las monedas
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
        // 1) Animación de pellets
        for s in self.sprites.iter_mut() {
            if s.kind == SpriteKind::Pellet {
                s.anim_time += dt;
                if s.anim_time > 0.5 {
                    s.anim_time = 0.0;
                    s.anim_frame = (s.anim_frame + 1) % 2;
                }
            }
        }

        // 2) IA de fantasmas con dispersión y separación
        let ghost_positions: Vec<(usize, f32, f32)> = self
            .sprites
            .iter()
            .enumerate()
            .filter_map(|(i, s)| if s.kind == SpriteKind::Ghost { Some((i, s.x, s.y)) } else { None })
            .collect();

        let scatter_r = 1.6_f32; // offset alrededor del jugador
        let sep_r = 0.9_f32; // separación entre fantasmas
        let speed = 1.35_f32;

        let mut rng = rand::thread_rng();

        for (k, (gi, gx, gy)) in ghost_positions.iter().enumerate() {
            // Animación simple del fantasma
            if let Some(gs) = self.sprites.get_mut(*gi) {
                gs.anim_time += dt;
                if gs.anim_time > 0.3 {
                    gs.anim_time = 0.0;
                    gs.anim_frame = (gs.anim_frame + 1) % 2;
                }
            }

            // Objetivo desplazado en círculo alrededor del jugador (diferente por fantasma)
            let angle = self.time * 0.6 + (k as f32) * 1.2566371; // ~2π/5
            let target_x = self.player.x + angle.cos() * scatter_r;
            let target_y = self.player.y + angle.sin() * scatter_r;

            // Dirección hacia el objetivo
            let mut vx = target_x - gx;
            let mut vy = target_y - gy;
            let mut len = (vx * vx + vy * vy).sqrt().max(1e-4);
            vx /= len;
            vy /= len;

            // Fuerza de separación de otros fantasmas
            let mut repx = 0.0;
            let mut repy = 0.0;
            for (j, (_oj_i, ox, oy)) in ghost_positions.iter().enumerate() {
                if j == k {
                    continue;
                }
                let dx = gx - ox;
                let dy = gy - oy;
                let d2 = dx * dx + dy * dy;
                if d2 < sep_r * sep_r {
                    let d = d2.sqrt().max(1e-3);
                    let force = (sep_r - d) / sep_r; // 0..1
                    repx += dx / d * force;
                    repy += dy / d * force;
                }
            }

            // Jitter aleatorio
            let jx = rng.gen_range(-0.2..0.2);
            let jy = rng.gen_range(-0.2..0.2);

            // Combinar y normalizar
            let mut fx = vx + 1.2 * repx + jx;
            let mut fy = vy + 1.2 * repy + jy;
            len = (fx * fx + fy * fy).sqrt().max(1e-4);
            fx /= len;
            fy /= len;

            // Movimiento con colisiones
            let nx = gx + fx * speed * dt;
            let ny = gy + fy * speed * dt;

            if let Some(gs) = self.sprites.get_mut(*gi) {
                if !is_wall_level(&self.level, nx, gs.y) {
                    gs.x = nx;
                }
                if !is_wall_level(&self.level, gs.x, ny) {
                    gs.y = ny;
                }
            }
        }
    }

    fn check_collisions_and_pickups(&mut self) {
        // 1) Recolección de pellets (pellets pequeños -> radio reducido)
        let pickup_r2 = 0.18f32 * 0.18f32;

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

        // 2) Colisión con fantasmas -> pierde vida
        if self.invincible_time <= 0.0 && self.mode == Mode::Playing {
            let hit_r2 = 0.30f32 * 0.30f32;
            let mut hit = false;

            for s in self.sprites.iter() {
                if s.kind == SpriteKind::Ghost {
                    let dx = self.player.x - s.x;
                    let dy = self.player.y - s.y;
                    let d2 = dx * dx + dy * dy;
                    if d2 < hit_r2 {
                        hit = true;
                        break;
                    }
                }
            }

            if hit {
                self.lives -= 1;
                self.audio.play_sfx("assets/sfx/hit.wav");

                if self.lives > 0 {
                    // Respawn con invulnerabilidad
                    let (px, py) = self.level.spawn;
                    self.player.x = px as f32 + 0.5;
                    self.player.y = py as f32 + 0.5;
                    self.invincible_time = 2.0;
                } else {
                    // Game Over
                    self.mode = Mode::GameOver;
                    self.death_anim_t = 0.0;
                    self.audio.play_sfx("assets/sfx/game_over.wav");
                }
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
            Mode::Paused => self.render_paused(frame, w, h),
            Mode::Win => self.render_win(frame, w, h),
            Mode::GameOver => self.render_game_over(frame, w, h),
        }
    }

    fn render_menu(&mut self, frame: &mut [u8], w: i32, h: i32) {
        fill(frame, w, h, 0x10, 0x10, 0x18);
        draw_text_small(frame, w, h, 16, 16, "PACMAN 3D - Raycaster", [255, 230, 0, 255]);
        draw_text_small(frame, w, h, 16, 40, "Selecciona un nivel:", [200, 200, 200, 255]);
        draw_text_small(frame, w, h, 16, 60, "[1] Nivel 1", [180, 220, 255, 255]);
        draw_text_small(frame, w, h, 16, 75, "[2] Nivel 2", [180, 220, 255, 255]);
        draw_text_small(frame, w, h, 16, 90, "[3] Nivel 3", [180, 220, 255, 255]);
        draw_text_small(
            frame,
            w,
            h,
            16,
            120,
            "Controles: W/S mover, Q/E o Flechas rotar, Mouse rota, P pausar",
            [180, 180, 180, 255],
        );
    }

    fn render_win(&mut self, frame: &mut [u8], w: i32, h: i32) {
        fill(frame, w, h, 0, 40, 0);
        draw_text_small(frame, w, h, 16, 16, "¡Nivel completado!", [255, 255, 255, 255]);
        draw_text_small(
            frame,
            w,
            h,
            16,
            40,
            "Presiona Enter para volver al menu",
            [200, 200, 200, 255],
        );
    }

    fn render_game(&mut self, frame: &mut [u8], w: i32, h: i32) {
        render_scene(frame, w, h, &self.level, &self.player, &self.sprites, &mut self.depth);

        // HUD
        let fps_txt = format!("FPS: {:.0}", self.fps);
        draw_text_small(frame, w, h, 6, 6, &fps_txt, [255, 255, 255, 255]);

        // Monedas (recogidas / total) y faltantes
        let collected = self.total_pellets.saturating_sub(self.pellets_remaining);
        let coins_txt = format!("Monedas: {}/{}", collected, self.total_pellets);
        draw_text_small(frame, w, h, 6, 20, &coins_txt, [255, 230, 0, 255]);

        let left_txt = format!("Faltan: {}", self.pellets_remaining);
        draw_text_small(frame, w, h, 6, 34, &left_txt, [200, 200, 200, 255]);

        // Vidas
        let lives_txt = format!("Vidas: {}", self.lives.max(0));
        draw_text_small(frame, w, h, 6, 50, &lives_txt, [255, 100, 100, 255]);
        for i in 0..self.lives.max(0) {
            rect_fill(frame, w, h, 70 + i * 8, 50, 6, 6, [220, 40, 40, 255]);
        }

        // Efecto de invulnerabilidad (flash sutil)
        if self.invincible_time > 0.0 {
            let a = ((self.invincible_time * 10.0).sin().abs() * 60.0) as u8;
            rect_fill(frame, w, h, 0, 0, w, h, [255, 255, 255, a]);
        }

        // Minimap
        self.render_minimap(frame, w, h);
    }

    fn render_paused(&mut self, frame: &mut [u8], w: i32, h: i32) {
        // Dibuja la escena congelada y un overlay de pausa
        self.render_game(frame, w, h);
        // Overlay semitransparente
        rect_fill(frame, w, h, 0, 0, w, h, [0, 0, 0, 140]);
        draw_text_small(frame, w, h, w / 2 - 30, h / 2 - 10, "PAUSA", [255, 255, 255, 255]);
        draw_text_small(
            frame,
            w,
            h,
            w / 2 - 90,
            h / 2 + 10,
            "P: continuar   Enter: menu",
            [220, 220, 220, 255],
        );
    }

    fn render_game_over(&mut self, frame: &mut [u8], w: i32, h: i32) {
        // Fondo oscuro
        fill(frame, w, h, 10, 0, 0);

        // Fade-in negro con tiempo
        let t = self.death_anim_t.min(2.0) / 2.0; // 0..1 en 2s
        let alpha = (t * 220.0) as u8;
        rect_fill(frame, w, h, 0, 0, w, h, [0, 0, 0, alpha]);

        draw_text_small(frame, w, h, 16, 16, "GAME OVER", [255, 255, 255, 255]);
        draw_text_small(frame, w, h, 16, 40, "Presiona R para reintentar", [200, 200, 200, 255]);
        draw_text_small(frame, w, h, 16, 55, "Presiona Enter para menu", [200, 200, 200, 255]);
    }

    fn render_minimap(&self, frame: &mut [u8], w: i32, h: i32) {
        let scale = 4;
        let pad = 6;
        let map_w = self.level.w as i32 * scale;
        let map_h = self.level.h as i32 * scale;

        let origin_x = w - map_w - pad;
        let origin_y = pad;

        rect_fill(
            frame,
            w,
            h,
            origin_x - 2,
            origin_y - 2,
            map_w + 4,
            map_h + 4,
            [0, 0, 0, 180],
        );

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

        // Fantasmas en el minimapa
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
        line(
            frame,
            w,
            h,
            px as i32,
            py as i32,
            (px + dx) as i32,
            (py + dy) as i32,
            [255, 255, 255, 255],
        );
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