use crate::game::{wall_color, Player};
// No importamos Level aquí para evitar conflictos

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
    // Cielo y piso
    draw_ceiling_floor(frame, w, h);

    // Raycast de paredes
    for x in 0..w {
        let camera_x = 2.0 * x as f32 / w as f32 - 1.0;
        let ray_dir_x = player.dir_x + player.plane_x * camera_x;
        let ray_dir_y = player.dir_y + player.plane_y * camera_x;

        let mut map_x = player.x as i32;
        let mut map_y = player.y as i32;

        let delta_dist_x = if ray_dir_x == 0.0 { f32::INFINITY } else { (1.0 / ray_dir_x).abs() };
        let delta_dist_y = if ray_dir_y == 0.0 { f32::INFINITY } else { (1.0 / ray_dir_y).abs() };

        let (step_x, mut side_dist_x) = if ray_dir_x < 0.0 {
            let s = -1;
            let sd = (player.x - map_x as f32) * delta_dist_x;
            (s, sd)
        } else {
            let s = 1;
            let sd = (map_x as f32 + 1.0 - player.x) * delta_dist_x;
            (s, sd)
        };
        let (step_y, mut side_dist_y) = if ray_dir_y < 0.0 {
            let s = -1;
            let sd = (player.y - map_y as f32) * delta_dist_y;
            (s, sd)
        } else {
            let s = 1;
            let sd = (map_y as f32 + 1.0 - player.y) * delta_dist_y;
            (s, sd)
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
    render_sprites(frame, w, h, level, player, sprites, depth);
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
    _level: &crate::level::Level,
    p: &Player,
    sprites: &[crate::sprites::Sprite],
    depth: &DepthBuffer,
) {
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

        let sprite_h = (h as f32 / transform_y).abs() as i32;
        let draw_start_y = (-sprite_h / 2 + h / 2).max(0);
        let draw_end_y = (sprite_h / 2 + h / 2).min(h - 1);

        let sprite_w = (h as f32 / transform_y).abs() as i32;
        let draw_start_x = (-sprite_w / 2 + sprite_screen_x).max(0);
        let draw_end_x = (sprite_w / 2 + sprite_screen_x).min(w - 1);

        let color = match s.kind {
            crate::sprites::SpriteKind::Pellet => {
                if s.anim_frame == 0 {
                    [255, 255, 0, 255]
                } else {
                    [255, 210, 0, 255]
                }
            }
            crate::sprites::SpriteKind::Ghost => {
                if s.anim_frame == 0 {
                    [255, 80, 80, 230]
                } else {
                    [255, 140, 140, 230]
                }
            }
        };

        for stripe in draw_start_x..=draw_end_x {
            let depth_x = depth.cols[stripe as usize];
            if transform_y < depth_x {
                for y in draw_start_y..=draw_end_y {
                    let cy = (y - (h / 2)) as f32 / (sprite_h as f32 / 2.0);
                    let cx = (stripe - sprite_screen_x) as f32 / (sprite_w as f32 / 2.0);
                    let r2 = cx * cx + cy * cy;
                    if r2 <= 1.0 {
                        let shade = ((1.2 - transform_y * 0.1).clamp(0.5, 1.0) * 255.0) as u8;
                        let idx = ((y * w + stripe) * 4) as usize;
                        frame[idx] = (color[0] as u16 * shade as u16 / 255) as u8;
                        frame[idx + 1] = (color[1] as u16 * shade as u16 / 255) as u8;
                        frame[idx + 2] = (color[2] as u16 * shade as u16 / 255) as u8;
                        frame[idx + 3] = color[3];
                    }
                }
            }
        }
    }
}