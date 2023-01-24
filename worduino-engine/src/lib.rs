#![no_std]

#![allow(incomplete_features)]
#![feature(generic_const_exprs)]

extern crate avr_progmem;
use avr_progmem::progmem;
use avr_progmem::wrapper::ProgMem;

pub const SCREEN_WIDTH: u8 = 128;
pub const SCREEN_HEIGHT: u8 = 64;

pub const LEVEL_WIDTH: u8 = 11;
pub const LEVEL_HEIGHT: u8 = 6;

#[derive(Clone, Copy)]
pub struct Walls {
    pub vertical_walls: [u16; LEVEL_HEIGHT as usize],
    pub horizontal_walls: [u8; LEVEL_WIDTH as usize],
}

impl From<WallsData> for Walls {
    fn from(walls_data: WallsData) -> Self {
        Walls {
            vertical_walls: mirror_v(walls_data.vertical_walls),
            horizontal_walls: mirror_h(walls_data.horizontal_walls),
        }
    }
}

#[derive(Clone, Copy)]
pub struct WallsData {
    pub vertical_walls: [u8; LEVEL_HEIGHT as usize],
    pub horizontal_walls: [u8; ((LEVEL_WIDTH + 1) / 2) as usize],
}

const fn reverse_bits(mut x: u8) -> u8 {
    let mut y: u8 = 0;
    let mut i = 7;
    while i != 0 {
        if x & 0x01 != 0 {
            y |= 0x80;
        }
        y = y.rotate_left(1);
        x >>= 1;
        i -= 1;
    }
    y
}

const fn mirror_v<const N: usize> (walls: [u8; N]) -> [u16; N] {
    let mut r: [u16; N] = [0; N];
    let mut i = 0;
    while i != walls.len() {
        let x = walls[i];
        let y = reverse_bits(x) >> 2;
        r[i] = ((y as u16) << 5) | (x as u16);
        i += 1;
    }
    r
}

const fn mirror_h<const N: usize> (walls: [u8; N]) -> [u8; 2 * N - 1] {
    let mut r: [u8; 2 * N - 1] = [0; 2 * N - 1];
    let mut i = 0;
    while i != r.len() {
        let j = if i < N { i } else { 2 * N - (i + 2) };
        r[i] = walls[j];
        i += 1;
    }
    r
}

progmem!{
    pub static progmem PLAYER_BULLET: [u8; 4] = [0x55, 0xaa, 0x55, 0xaa];
    pub static progmem MONSTER_BULLET: [u8; 4] = [0xaa, 0x55, 0xaa, 0x55];

    pub static progmem LEVEL: WallsData = WallsData {
        horizontal_walls: [0x00; 6],
        vertical_walls: [0x00; 6],
    };
}

pub trait Peripherals {
    fn get_stripe(&self, x: u8, stripe: u8) -> u8;
    fn set_stripe(&mut self, x: u8, stripe: u8, val: u8);
    fn get_button(&self) -> bool;

    fn set_pixel(&mut self, x: u8, y: u8, val: bool) {
        let stripe = y / 8;
        let offset = y - stripe * 8;
        let old = self.get_stripe(x, stripe);
        let mask = 1 << offset;
        let new = if val { old | mask } else { old & !mask };
        self.set_stripe(x, stripe, new)
    }
}

pub struct Engine<P: Peripherals> {
    pub peripherals: P,
    state: State,
}

impl<P: Peripherals> Engine<P> {
    pub fn new(peripherals: P) -> Engine<P> {
        let mut player = Player::new();
        Engine{
            peripherals: peripherals,
            state: State::Playing{ level_state: LevelState::new(&mut player) },
        }
    }

    pub fn step(&mut self) {
        clear_screen(&mut self.peripherals);

        match &self.state {
            State::GameOver{ score } => {
            },

            State::Playing{ mut level_state } => {
                self.state = level_state.step(self);
            }
        }
    }
}

fn fill_screen(p: &mut impl Peripherals, value: u8) {
    for x in 0..SCREEN_WIDTH {
        for stripe in 0..SCREEN_HEIGHT / 8 {
            p.set_stripe(x, stripe, value)
        }
    }
}

fn clear_screen(p: &mut impl Peripherals) {
    fill_screen(p, 0x00)
}

fn draw_sprite(p: &mut impl Peripherals, sprite: ProgMem<[u8]>, pos: (u8, u8)) {
    let (x0, y0) = pos;

    for i in 0..sprite.len() {
        let dx = i as u8;
        let mut col = sprite.load_at(i);
        for dy in 0..8 {
            p.set_pixel(x0 + dx, y0 + dy, col & 1 != 0);
            col >>= 1;
        }
    }
}

#[derive(Clone, Copy)]
struct Entity {
    pos: (u8, u8),
}

enum State {
    Playing {
        level_state: LevelState,
    },
    GameOver {
        score: u16,
    },
}

#[derive(Clone, Copy)]
struct LevelState {
    walls: Walls,
    player: Player,
    monsters: [Option<BasicMonster>; 8],
}

impl LevelState {
    fn new(player: &mut Player) -> Self {
        let layout = LEVEL.load();

        LevelState {
            walls: layout.into(),
            player: *player,
            monsters: [None; 8],
        }
    }

    fn draw<P: Peripherals>(&mut self, engine: &mut Engine<P>) {
        self.player.draw(&mut engine.peripherals);
        for monster in self.monsters.iter() {
            if let Some(monster) = monster {
                monster.draw(&mut engine.peripherals);
            }
        }
    }

    fn step<P: Peripherals>(&mut self, engine: &mut Engine<P>) -> State {
        self.draw(engine);

        let mut player = self.player;
        player.action();

        let mut monsters = self.monsters;
        let mut monster_count: u8 = 0;
        for monster_slot in monsters.iter_mut() {
            if monster_slot.is_some() {
                monster_count += 1;
            }
        }

        self.player = player;
        self.monsters = monsters;

        if self.player.lives == u8::MAX {
            return State::GameOver{ score: 0 };
        }

        if monster_count == 0 {
            State::Playing{ level_state: Self::new(&mut self.player) }
        } else {
            State::Playing{ level_state: *self }
        }
    }
}

#[derive(Clone, Copy)]
struct BasicMonster {
    bullet: Option<Entity>,
}

impl BasicMonster {
    fn draw(&self, p: &mut impl Peripherals) {
        if let Some(ref e) = self.bullet {
            draw_sprite(p, MONSTER_BULLET, e.pos);
        }
    }
}

#[derive(Clone, Copy)]
struct Player {
    e: Entity,
    lives: u8,
    score: u16,
    bullet: Option<Entity>,
}

impl Player {
    fn start_pos() -> Entity {
        Entity {
            pos: (40, 40),
        }
    }

    fn new() -> Player {
        Player {
            e: Self::start_pos(),
            lives: 2,
            score: 0,
            bullet: None,
        }
    }

    fn draw(&self, p: &mut impl Peripherals) {
        let Player{ bullet, .. } = self;
        if let Some(ref e) = bullet {
            draw_sprite(p, PLAYER_BULLET, e.pos);
        }
    }

    fn action(&mut self) {
    }
}
