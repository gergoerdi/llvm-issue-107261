#![feature(abi_avr_interrupt)]
#![feature(core_intrinsics)]
#![feature(asm_experimental_arch)]

#![no_std]
#![no_main]

extern crate avr_std_stub;
extern crate worduino_engine as worduino;

use worduino::*;

struct ArduboyPeripherals {
    pub framebuffer: [[u8; SCREEN_WIDTH as usize]; SCREEN_HEIGHT as usize / 8],
}

impl ArduboyPeripherals {
    fn new() -> ArduboyPeripherals {
        ArduboyPeripherals {
            framebuffer: [[0; SCREEN_WIDTH as usize]; SCREEN_HEIGHT as usize / 8],
        }
    }
}

impl Peripherals for ArduboyPeripherals {
    fn get_button(&self) -> bool {
        false
    }

    fn get_stripe(&self, x: u8, stripe: u8) -> u8 {
        0x00
    }

    fn set_stripe(&mut self, x: u8, stripe: u8, val: u8) {
        self.framebuffer[stripe as usize][x as usize] = val;
    }
}

#[no_mangle]
pub extern fn main() {
    let peripherals = ArduboyPeripherals::new();
    let mut engine = Engine::new(peripherals);

    loop {
        engine.step();
    }
}
