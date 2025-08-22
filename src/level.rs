pub struct Level {
    pub w: i32,
    pub h: i32,
    pub map: Vec<i32>,
    pub spawn: (i32, i32),
    pub ghost_count: usize,
}

impl Level {
    pub fn tile(&self, x: i32, y: i32) -> i32 {
        if x < 0 || y < 0 || x >= self.w || y >= self.h {
            1
        } else {
            self.map[(y * self.w + x) as usize]
        }
    }
}

pub fn get_level(idx: usize) -> Level {
    match idx {
        0 => level1(),
        1 => level2(),
        _ => level3(),
    }
}

// Nivel 1: sencillo
fn level1() -> Level {
    let w = 24;
    let h = 16;
    let mut map = vec![0; (w * h) as usize];

    // Bordes
    for x in 0..w {
        map[(0 * w + x) as usize] = 1;
        map[((h - 1) * w + x) as usize] = 1;
    }
    for y in 0..h {
        map[(y * w + 0) as usize] = 1;
        map[(y * w + (w - 1)) as usize] = 1;
    }
    // Algunas paredes internas
    for x in 3..w - 3 {
        map[(5 * w + x) as usize] = if x % 2 == 0 { 2 } else { 3 };
    }
    for y in 3..h - 3 {
        map[(y * w + 8) as usize] = 4;
        map[(y * w + 15) as usize] = 5;
    }

    Level {
        w,
        h,
        map,
        spawn: (2, 2),
        ghost_count: 3,
    }
}

// Nivel 2: laberinto medio
fn level2() -> Level {
    let w = 28;
    let h = 18;
    let mut map = vec![0; (w * h) as usize];

    for x in 0..w {
        map[(0 * w + x) as usize] = 2;
        map[((h - 1) * w + x) as usize] = 2;
    }
    for y in 0..h {
        map[(y * w + 0) as usize] = 2;
        map[(y * w + (w - 1)) as usize] = 2;
    }
    for y in (2..h - 2).step_by(2) {
        for x in 2..w - 2 {
            if x % 4 != 0 {
                map[(y * w + x) as usize] = if (x + y) % 3 == 0 { 3 } else { 4 };
            }
        }
    }
    for x in (3..w - 3).step_by(2) {
        for y in 3..h - 3 {
            if y % 3 != 0 {
                map[(y * w + x) as usize] = 5;
            }
        }
    }

    Level {
        w,
        h,
        map,
        spawn: (1, 1),
        ghost_count: 5,
    }
}

// Nivel 3: más grande y denso
fn level3() -> Level {
    let w = 32;
    let h = 20;
    let mut map = vec![0; (w * h) as usize];

    for x in 0..w {
        map[(0 * w + x) as usize] = 3;
        map[((h - 1) * w + x) as usize] = 3;
    }
    for y in 0..h {
        map[(y * w + 0) as usize] = 3;
        map[(y * w + (w - 1)) as usize] = 3;
    }
    for y in 2..h - 2 {
        for x in 2..w - 2 {
            if (x + y) % 2 == 0 && (x % 6 != 0) {
                map[(y * w + x) as usize] = if x % 3 == 0 { 4 } else { 5 };
            }
        }
    }
    // pasillos
    for x in 4..w - 4 {
        map[((h / 2) * w + x) as usize] = 1;
    }
    for y in 4..h - 4 {
        map[(y * w + w / 3) as usize] = 2;
        map[(y * w + 2 * w / 3) as usize] = 2;
    }

    Level {
        w,
        h,
        map,
        spawn: (2, 2),
        ghost_count: 7,
    }
}