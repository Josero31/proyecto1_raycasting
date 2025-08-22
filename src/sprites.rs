#[derive(Copy, Clone, PartialEq, Eq)]
pub enum SpriteKind {
    Pellet,
    Ghost,
}

pub struct Sprite {
    pub x: f32,
    pub y: f32,
    pub kind: SpriteKind,
    pub anim_frame: usize,
    pub anim_time: f32,
}

impl Sprite {
    pub fn new(x: f32, y: f32, kind: SpriteKind) -> Self {
        Self {
            x, y, kind,
            anim_frame: 0,
            anim_time: 0.0,
        }
    }
}