use crate::game::{wall_color, Player};

// Profundidad por columna
pub struct DepthBuffer {
    pub cols: Vec<f32>,
}
impl DepthBuffer {
    pub fn new(width: usize) -> Self {
        Self { cols: vec![f32::INFINITY; width] }
    }
}

pub fn render_scene(
    frame: &mut [u8],
    w: i32,
    h: i32,
    level: &crate::level::Level,
    player: &Player,
    sprites: &[crate::sprites::Sprite],
    depth: &mut DepthBuffer,
) {
    // Cielo y piso planos
    draw_ceiling_floor(frame, w, h);

    // Raycast de paredes sólidas (sin texturas)
    for x in 0..w {
        let camera_x = 2.0 * x as f32 / w as f32 - 1.0;
        let ray_dir_x = player.dir_x + player.plane_x * camera_x;
        let ray_dir_y = player.dir_y + player.plane_y * camera_x;

        let mut map_x = player.x as i32;
        let mut map_y = player.y as i32;

        let delta_dist_x = if ray_dir_x == 0.0 { f32::INFINITY } else { (1.0 / ray_dir_x).abs() };
        let delta_dist_y = if ray_dir_y == 0.0 { f32::INFINITY } else { (1.0 / ray_dir_y).abs() };

        let (step_x, mut side_dist_x) = if ray_dir_x < 0.0 {
            (-1, (player.x - map_x as f32) * delta_dist_x)
        } else {
            (1, (map_x as f32 + 1.0 - player.x) * delta_dist_x)
        };
        let (step_y, mut side_dist_y) = if ray_dir_y < 0.0 {
            (-1, (player.y - map_y as f32) * delta_dist_y)
        } else {
            (1, (map_y as f32 + 1.0 - player.y) * delta_dist_y)
        };

        let mut hit = 0;
        let mut side = 0; // 0: x, 1: y
        while hit == 0 {
            if side_dist_x < side_dist_y {
                side_dist_x += delta_dist_x;
                map_x += step_x;
                side = 0;
            } else {
                side_dist_y += delta_dist_y;
                map_y += step_y;
                side = 1;
            }
            if map_x < 0 || map_y < 0 || map_x >= level.w || map_y >= level.h {
                hit = -1;
                break;
            }
            let tile = level.tile(map_x, map_y);
            if tile > 0 {
                hit = tile;
            }
        }

        let perp_wall_dist = if hit == -1 {
            1e6
        } else if side == 0 {
            (map_x as f32 - player.x + (1 - step_x) as f32 / 2.0) / ray_dir_x
        } else {
            (map_y as f32 - player.y + (1 - step_y) as f32 / 2.0) / ray_dir_y
        }
        .abs()
        .max(1e-4);

        let line_height = (h as f32 / perp_wall_dist) as i32;
        let mut draw_start = -line_height / 2 + h / 2;
        if draw_start < 0 {
            draw_start = 0;
        }
        let mut draw_end = line_height / 2 + h / 2;
        if draw_end >= h {
            draw_end = h - 1;
        }

        let mut color = if hit > 0 { wall_color(hit) } else { [0, 0, 0, 255] };
        if side == 1 {
            color[0] = (color[0] as f32 * 0.7) as u8;
            color[1] = (color[1] as f32 * 0.7) as u8;
            color[2] = (color[2] as f32 * 0.7) as u8;
        }

        for y in draw_start..=draw_end {
            let idx = ((y * w + x) * 4) as usize;
            frame[idx..idx + 4].copy_from_slice(&color);
        }

        depth.cols[x as usize] = perp_wall_dist;
    }

    // Render de sprites
    render_sprites(frame, w, h, player, sprites, depth);
}

fn draw_ceiling_floor(frame: &mut [u8], w: i32, h: i32) {
    let half = h / 2;
    for y in 0..half {
        for x in 0..w {
            let idx = ((y * w + x) * 4) as usize;
            frame[idx] = 40;
            frame[idx + 1] = 60;
            frame[idx + 2] = 120;
            frame[idx + 3] = 255;
        }
    }
    for y in half..h {
        for x in 0..w {
            let idx = ((y * w + x) * 4) as usize;
            frame[idx] = 40;
            frame[idx + 1] = 40;
            frame[idx + 2] = 40;
            frame[idx + 3] = 255;
        }
    }
}

fn render_sprites(
    frame: &mut [u8],
    w: i32,
    h: i32,
    p: &Player,
    sprites: &[crate::sprites::Sprite],
    depth: &DepthBuffer,
) {
    // Ordenar por distancia (lejano a cercano)
    let mut order: Vec<(usize, f32)> = sprites
        .iter()
        .enumerate()
        .map(|(i, s)| {
            let dx = s.x - p.x;
            let dy = s.y - p.y;
            (i, dx * dx + dy * dy)
        })
        .collect();
    order.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());

    let inv_det = 1.0 / (p.plane_x * p.dir_y - p.dir_x * p.plane_y);

    for (i, _dist2) in order {
        let s = &sprites[i];
        let sprite_x = s.x - p.x;
        let sprite_y = s.y - p.y;

        let transform_x = inv_det * (p.dir_y * sprite_x - p.dir_x * sprite_y);
        let transform_y = inv_det * (-p.plane_y * sprite_x + p.plane_x * sprite_y);

        if transform_y <= 0.01 {
            continue;
        }

        let sprite_screen_x = (w as f32 / 2.0 * (1.0 + transform_x / transform_y)) as i32;

        // Escala por tipo: pellets más pequeños, fantasmas casi tamaño completo
        let scale = match s.kind {
            crate::sprites::SpriteKind::Pellet => 0.35, // monedas más pequeñas
            crate::sprites::SpriteKind::Ghost => 0.9,   // fantasmas grandes
        };

        let sprite_h = ((h as f32 / transform_y) * scale).abs() as i32;
        let draw_start_y = (-sprite_h / 2 + h / 2).max(0);
        let draw_end_y = (sprite_h / 2 + h / 2).min(h - 1);

        let sprite_w = sprite_h; // cuadrado
        let draw_start_x = (-sprite_w / 2 + sprite_screen_x).max(0);
        let draw_end_x = (sprite_w / 2 + sprite_screen_x).min(w - 1);

        for stripe in draw_start_x..=draw_end_x {
            if transform_y >= depth.cols[stripe as usize] {
                continue;
            }

            // Coordenadas normalizadas en X para la columna dentro del sprite [-1, 1]
            let nx = (stripe - sprite_screen_x) as f32 / (sprite_w as f32 / 2.0);

            // Para avanzar en Y en el sprite
            let mut tpos = (draw_start_y - h / 2 + sprite_h / 2) as f32 / (sprite_h as f32); // [0..1] al empezar
            let tstep = 1.0 / sprite_h.max(1) as f32;

            for y in draw_start_y..=draw_end_y {
                // Coordenada Y normalizada dentro del sprite:
                // cy en [-1,1], ty en [0,1]
                let cy = (y - (h / 2)) as f32 / (sprite_h as f32 / 2.0);
                let ty = tpos; // 0 en la parte superior del sprite, 1 en la inferior
                tpos += tstep;

                let mut write = false;
                let mut rgba = [0u8, 0u8, 0u8, 0u8];

                match s.kind {
                    crate::sprites::SpriteKind::Pellet => {
                        // Círculo pequeño
                        let r2 = nx * nx + cy * cy;
                        if r2 <= 1.0 {
                            let base = [255, 230, 0, 255];
                            // leve sombreado por distancia
                            let shade = ((1.2 - transform_y * 0.1).clamp(0.5, 1.0) * 255.0) as u8;
                            rgba = [
                                (base[0] as u16 * shade as u16 / 255) as u8,
                                (base[1] as u16 * shade as u16 / 255) as u8,
                                (base[2] as u16 * shade as u16 / 255) as u8,
                                255,
                            ];
                            write = true;
                        }
                    }
                    crate::sprites::SpriteKind::Ghost => {
                        // Figura de fantasma procedimental:
                        // - cúpula superior (semicírculo)
                        // - cuerpo rectangular
                        // - borde inferior ondulado (3 “picos”)
                        // Coordenadas: nx [-1,1], ty [0,1]
                        let mut inside = false;

                        // Cúpula superior: círculo de radio r con centro (0, r) en espacio ty
                        let r = 0.45;
                        if ty <= r {
                            let dx = nx;
                            let dy = ty - r;
                            if dx * dx + dy * dy <= r * r {
                                inside = true;
                            }
                        }
                        // Cuerpo
                        if ty > r && ty <= 0.9 && nx.abs() <= 0.85 {
                            inside = true;
                        }
                        // Borde inferior ondulado (tres semicúpulas)
                        if ty > 0.9 && ty <= 1.0 {
                            let centers = [-0.5f32, 0.0, 0.5];
                            let rr = 0.12;
                            for cx in centers {
                                let dx = nx - cx;
                                let dy = ty - 0.9;
                                if dx * dx + dy * dy <= rr * rr {
                                    inside = true;
                                    break;
                                }
                            }
                        }

                        if inside {
                            // Color base animado (parpadeo leve usando anim_frame)
                            let base = if s.anim_frame == 0 {
                                [255, 120, 120, 235]
                            } else {
                                [255, 150, 150, 235]
                            };
                            // Ojos: dos círculos blancos con pupilas azules
                            // Posiciones relativas
                            let eye_y = 0.35;
                            let eye_rx = 0.17;
                            let eye_lx = -0.17;
                            let eye_r = 0.12;
                            let pupil_r = 0.06;

                            // ¿Dentro del ojo izquierdo o derecho?
                            let dlx = nx - eye_lx;
                            let dly = ty - eye_y;
                            let drx = nx - eye_rx;
                            let dry = ty - eye_y;

                            let mut col = base;

                            if dlx * dlx + dly * dly <= eye_r * eye_r
                                || drx * drx + dry * dry <= eye_r * eye_r
                            {
                                // blanco del ojo
                                col = [250, 250, 250, 255];
                                // Pupilas centradas
                                let pl = dlx * dlx + dly * dly <= pupil_r * pupil_r;
                                let pr = drx * drx + dry * dry <= pupil_r * pupil_r;
                                if pl || pr {
                                    col = [60, 100, 255, 255];
                                }
                            }

                            // Sombreado por distancia
                            let shade = ((1.1 - transform_y * 0.08).clamp(0.5, 1.0) * 255.0) as u8;
                            rgba = [
                                (col[0] as u16 * shade as u16 / 255) as u8,
                                (col[1] as u16 * shade as u16 / 255) as u8,
                                (col[2] as u16 * shade as u16 / 255) as u8,
                                col[3],
                            ];
                            write = true;
                        }
                    }
                }

                if write {
                    let idx = ((y * w + stripe) * 4) as usize;
                    frame[idx] = rgba[0];
                    frame[idx + 1] = rgba[1];
                    frame[idx + 2] = rgba[2];
                    frame[idx + 3] = 255;
                }
            }
        }
    }
}