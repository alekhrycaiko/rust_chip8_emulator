use sdl2;
extern crate rand;
use crate::audio;
use crate::font::FontSet;
use crate::keyboard::Keyboard;
use rand::Rng;
const DISPLAY_WIDTH: usize = 64;
const DISPLAY_HEIGHT: usize = 32;
const CPU_MEMORY: usize = 4096;

pub struct Output<'a> {
    pub display_memory: &'a [[u8; DISPLAY_WIDTH]; DISPLAY_HEIGHT],
    pub display_changed: bool,
}

pub struct CPU {
    reg_v: [u8; 16],
    reg_i: usize, // special register; right most 12 bits are used.
    delay_timer: u8,
    sound_timer: u8,
    pc: u16,
    sp: usize,
    stack: [u16; 16],
    memory: [u8; CPU_MEMORY],
    pub keyboard: Keyboard,
    display_memory: [[u8; DISPLAY_WIDTH]; DISPLAY_HEIGHT],
    display_changed: bool,
    keycode: u8,
    keyboard_blocking: bool,
}

impl CPU {
    pub fn new(file_buffer: &[u8]) -> Self {
        let mut memory = [0x00; CPU_MEMORY];
        let font_set = FontSet::new();
        for i in 0..font_set.set.len() {
            memory[i] = font_set.set[i];
        }
        for (i, &byte) in file_buffer.iter().enumerate() {
            let address = 0x200 + i;
            if address < 4096 {
                memory[address] = byte;
            } else {
                break;
            }
        }
        CPU {
            reg_v: [0; 16],
            reg_i: 0,
            delay_timer: 0,
            sound_timer: 0,
            pc: 0x000,
            sp: 0,
            stack: [0x000; 16],
            memory,
            keyboard: Keyboard::new(),
            display_memory: [[0; DISPLAY_WIDTH]; DISPLAY_HEIGHT],
            display_changed: false,
            keycode: 0x0,
            keyboard_blocking: false,
        }
    }

    pub fn cycle(&mut self, sdl_context: &sdl2::Sdl, audio_driver: &audio::Audio) -> Output {
        if self.keyboard_blocking {
            while self.keyboard_blocking {
                self.keyboard.cycle(sdl_context);
                if self.keyboard.key_clicked > 0 && self.keyboard.key_clicked < 0x11 {
                    self.keyboard_blocking = false;
                    self.keycode = self.keyboard.key_clicked;
                }
            }
        } else {
            let _ = self.keyboard.cycle(sdl_context);
        }
        let opcode = self.get_opcode();
        self.handle_opcode(opcode);

        if self.delay_timer > 0 {
            self.delay_timer -= 1;
        }
        if self.sound_timer > 0 {
            audio_driver.play();
            self.sound_timer -= 1;
        }

        Output {
            display_memory: &self.display_memory,
            display_changed: self.display_changed,
        }
    }

    fn get_opcode(&mut self) -> u16 {
        let first_byte = self.memory[self.pc as usize];
        let second_byte = self.memory[self.pc as usize + 1];
        ((first_byte as u16) << 8) | second_byte as u16
    }

    pub fn handle_opcode(&mut self, opcode: u16) {
        let tuple_opcode = (
            (opcode & 0xf000) >> 12 as u8,
            (opcode & 0x0f00) >> 8 as u8,
            (opcode & 0x00f0) >> 4 as u8,
            (opcode & 0x000f) as u8,
        );
        self.pc = match tuple_opcode {
            (0x0, 0x0, 0xe, 0x0) => self.handle_cls(),
            (0x0, 0x0, 0xe, 0xe) => self.handle_ret(),
            (0x1, _, _, _) => self.handle_1nnn(opcode),
            (0x2, _, _, _) => self.handle_2nnn(opcode),
            (0x3, _, _, _) => self.handle_3xkk(opcode),
            (0x4, _, _, _) => self.handle_4xkk(opcode),
            (0x5, _, _, _) => self.handle_5xy0(opcode),
            (0x6, _, _, _) => self.handle_6xkk(opcode),
            (0x7, _, _, _) => self.handle_7xkk(opcode),
            (0x8, _, _, 0x0) => self.handle_8xy0(opcode),
            (0x8, _, _, 0x1) => self.handle_8xy1(opcode),
            (0x8, _, _, 0x2) => self.handle_8xy2(opcode),
            (0x8, _, _, 0x3) => self.handle_8xy3(opcode),
            (0x8, _, _, 0x4) => self.handle_8xy4(opcode),
            (0x8, _, _, 0x5) => self.handle_8xy5(opcode),
            (0x8, _, _, 0x6) => self.handle_8xy6(opcode),
            (0x8, _, _, 0x7) => self.handle_8xy7(opcode),
            (0x8, _, _, 0xe) => self.handle_8xye(opcode),
            (0x9, _, _, 0x0) => self.handle_9xy0(opcode),
            (0xa, _, _, _) => self.handle_annn(opcode),
            (0xb, _, _, _) => self.handle_bnnn(opcode),
            (0xc, _, _, _) => self.handle_cxkk(opcode),
            (0xd, _, _, _) => self.handle_dxyn(opcode),
            (0xe, _, 0x9, 0xe) => self.handle_ex9e(opcode),
            (0xe, _, 0xa, 0x1) => self.handle_exa1(opcode),
            (0xf, _, 0x0, 0x7) => self.handle_fx07(opcode),
            (0xf, _, 0x0, 0xa) => self.handle_fx0a(opcode),
            (0xf, _, 0x1, 0x5) => self.handle_fx15(opcode),
            (0xf, _, 0x1, 0x8) => self.handle_fx18(opcode),
            (0xf, _, 0x1, 0xe) => self.handle_fx1e(opcode),
            (0xf, _, 0x2, 0x9) => self.handle_fx29(opcode),
            (0xf, _, 0x3, 0x3) => self.handle_fx33(opcode),
            (0xf, _, 0x5, 0x5) => self.handle_fx55(opcode),
            (0xf, _, 0x6, 0x5) => self.handle_fx65(opcode),
            (_, _, _, _) => self.pc + 2,
        };
    }
    fn handle_1nnn(&mut self, opcode: u16) -> u16 {
        self.pc = opcode & 0x0fff;
        self.pc
    }

    fn handle_cls(&mut self) -> u16 {
        let memory = self.display_memory;
        for (y, col) in memory.iter().enumerate() {
            for (x, _) in col.iter().enumerate() {
                self.display_memory[y][x] = 0;
            }
        }
        self.display_changed = true;
        self.pc + 2
    }

    fn handle_ret(&mut self) -> u16 {
        self.sp -= 1;
        self.pc = self.stack[self.sp];
        self.pc
    }
    /**
    * Call subroutine at nnn.
    The interpreter increments the stack pointer, then puts the current PC on the top of the stack. The PC is then set to nnn.
    */
    fn handle_2nnn(&mut self, opcode: u16) -> u16 {
        self.stack[self.sp] = self.pc + 2;
        self.sp += 1;
        self.pc = get_nnn(opcode) as u16;
        self.pc
    }
    /*
     * Skip next instruction if Vx = kk.
     * The interpreter compares register Vx to kk, and if they are equal, increments the program counter by 2.
     */
    fn handle_3xkk(&mut self, opcode: u16) -> u16 {
        let kk = (opcode & 0x00ff) as u8;
        let vx = self.reg_v[get_x(opcode)] as u8;
        if vx == kk {
            return self.pc + 4;
        }
        self.pc + 2
    }
    /**
    * Skip next instruction if Vx != kk.
    The interpreter compares register Vx to kk, and if they are not equal, increments the program counter by 2.
    */
    fn handle_4xkk(&mut self, opcode: u16) -> u16 {
        let kk = get_kk(opcode);
        let x = get_x(opcode);
        if self.reg_v[x] as usize == kk {
            return self.pc + 2;
        }
        self.pc + 4
    }
    /**
     * Skip next instruction if Vx = Vy.
     * The interpreter compares register Vx to register Vy, and if they are equal, increments the program counter by 2.
     **/
    fn handle_5xy0(&mut self, opcode: u16) -> u16 {
        if self.reg_v[get_x(opcode)] == self.reg_v[get_y(opcode)] {
            return self.pc + 4;
        }
        self.pc + 2
    }

    /**
     * Set Vx = kk.
     * The interpreter puts the value kk into register Vx.
     */
    fn handle_6xkk(&mut self, opcode: u16) -> u16 {
        self.reg_v[get_x(opcode)] = get_kk(opcode) as u8;
        self.pc + 2
    }

    /**
     * Set Vx = Vx + kk.
     * Adds the value kk to the value of register Vx, then stores the result in V_x
     */
    fn handle_7xkk(&mut self, opcode: u16) -> u16 {
        let result = get_kk(opcode) as u16 + self.reg_v[get_x(opcode)] as u16;
        self.reg_v[get_x(opcode)] = result as u8;
        self.pc + 2
    }

    /**
     * Set Vx = Vy.
     * Stores the value of register Vy in register Vx.
     */
    fn handle_8xy0(&mut self, opcode: u16) -> u16 {
        self.reg_v[get_x(opcode)] = self.reg_v[get_y(opcode)];
        self.pc + 2
    }

    /**
    Set Vx = Vx OR Vy.
    Performs a bitwise OR on the values of Vx and Vy, then stores the result in Vx.
    */
    fn handle_8xy1(&mut self, opcode: u16) -> u16 {
        self.reg_v[get_x(opcode)] |= self.reg_v[get_y(opcode)];
        self.pc + 2
    }

    /**
     * Vx = Vx AND Vy
     */
    fn handle_8xy2(&mut self, opcode: u16) -> u16 {
        self.reg_v[get_x(opcode)] &= self.reg_v[get_y(opcode)];
        self.pc + 2
    }
    /**
     * Vx = XOR Vx Vy
     */
    fn handle_8xy3(&mut self, opcode: u16) -> u16 {
        self.reg_v[get_x(opcode)] ^= self.reg_v[get_y(opcode)];
        self.pc + 2
    }
    /**
     * Vx = Vx + Vy
     * If Vx > 255, set V_f to 1, otherwise set to 0.
     * Of the result, save only the top 8 bits.
     */
    fn handle_8xy4(&mut self, opcode: u16) -> u16 {
        let val = self.reg_v[get_x(opcode)] as u16 + self.reg_v[get_y(opcode)] as u16;
        if (val as usize) > 255 {
            self.reg_v[0x0f] = 1;
        } else {
            self.reg_v[0x0f] = 0;
        }
        self.reg_v[get_x(opcode)] = val as u8;
        self.pc + 2
    }
    /**
     * If V_x > V_y then V_flag = 1
     * Otherwise v_flag = 0
     * Then sets V_x = V_x - V_y
     */
    fn handle_8xy5(&mut self, opcode: u16) -> u16 {
        let v_x = self.reg_v[get_x(opcode)];
        let v_y = self.reg_v[get_y(opcode)];
        if v_x > v_y {
            self.reg_v[0x0f] = 1;
        } else {
            self.reg_v[0x0f] = 0;
        }
        self.reg_v[get_x(opcode)] = v_x.wrapping_sub(v_y);
        self.pc + 2
    }

    /**
     * If least significant bit of V_x is 1; then V_f is 1.
     * Otherwise, v_f is 0;
     * V_x is then divided by 2 and saved.
     */
    fn handle_8xy6(&mut self, opcode: u16) -> u16 {
        let v_x = self.reg_v[get_x(opcode)];
        let least_sig_bit = v_x & 0x1;
        if least_sig_bit == 1 {
            self.reg_v[0x0f] = 1;
        } else {
            self.reg_v[0x0f] = 0;
        }
        self.reg_v[get_x(opcode)] >>= 1;
        self.pc + 2
    }

    /**
     * If Vy > Vx, V_f is 1; otherwise V_f 0.
     * Then sets V_x = Vy - Vx
     */
    fn handle_8xy7(&mut self, opcode: u16) -> u16 {
        let v_y = self.reg_v[get_y(opcode)];
        let v_x = self.reg_v[get_x(opcode)];
        if v_y > v_x {
            self.reg_v[0x0f] = 1;
        } else {
            self.reg_v[0x0f] = 0;
        }
        self.reg_v[get_x(opcode)] = v_y.wrapping_sub(v_x);
        self.pc + 2
    }

    /**
     * If the most significant bit of Vx is 1, then set V_f to 1. Otherwise 0.
     * V_x following this is also multipled by 2.
     */
    fn handle_8xye(&mut self, opcode: u16) -> u16 {
        let v_x = self.reg_v[get_x(opcode)] & 0b1000_0000;
        let most_sig_bit = v_x >> 7;
        if most_sig_bit == 1 {
            self.reg_v[0x0f] = 1;
        } else {
            self.reg_v[0x0f] = 0;
        }
        self.reg_v[get_x(opcode)] <<= 1;
        self.pc + 2
    }

    /**
     * Skip next instr if Vx != Vy
     */
    fn handle_9xy0(&mut self, opcode: u16) -> u16 {
        if self.reg_v[get_x(opcode)] != self.reg_v[get_y(opcode)] {
            return self.pc + 4;
        }
        self.pc + 2
    }

    /**
     * Set value of register I to NNN
     */
    fn handle_annn(&mut self, opcode: u16) -> u16 {
        self.reg_i = get_nnn(opcode);
        self.pc + 2
    }
    /**
     * Jump to location nnn + v0
     */
    fn handle_bnnn(&mut self, opcode: u16) -> u16 {
        let pc = (self.reg_v[0] as usize) + get_nnn(opcode);
        pc as u16
    }

    /**
     * Vx = random byte AND kk
     * random byte is a random number from
     *  The interpreter generates a random number from 0 to 255, which is then ANDed with the value kk.
     *  The results are stored in Vx.      
     *  */
    fn handle_cxkk(&mut self, opcode: u16) -> u16 {
        let val: u8 = rand::thread_rng().gen();
        self.reg_v[get_x(opcode)] = val & get_kk(opcode) as u8;
        self.pc + 2
    }
    /**
     * Display n-byte sprite starting at memory location I, at (Vx, Vy)
     * Set V_f = collision
     * Interpreter should read  n bytes from memory starting at I.
     * These bytes are displayed as sprites on the screen at (Vx, Vy)
     * Sprites are XORed onto the screen.
     * If any pixels are erased in this XOR; set V_f to 1. Otherwise V_f is 0
     * If the sprite is positioned outside of the display; a wrap around
     * to the other side should occur.
     *
     */
    fn handle_dxyn(&mut self, opcode: u16) -> u16 {
        let n = get_n(opcode);
        self.reg_v[0x0f] = 0;
        for byte in 0..n {
            let y = (self.reg_v[get_y(opcode)] as usize + byte) % DISPLAY_HEIGHT;
            for bit in 0..8 {
                let sprite_value = (self.memory[self.reg_i + byte] >> (7 - bit)) & 1;
                // Referenced from Starhorne's implementation
                let x = (self.reg_v[get_x(opcode)] as usize + bit) % DISPLAY_WIDTH;
                self.reg_v[0x0f] |= self.display_memory[y][x] & sprite_value;
                self.display_memory[y][x] ^= sprite_value;
            }
        }
        self.display_changed = true;
        self.pc + 2
    }

    /**
     * Skip next instr if key with value of Vx is pressed.
     */
    fn handle_ex9e(&mut self, opcode: u16) -> u16 {
        let is_pressed = self.keyboard.is_key_pressed(self.reg_v[get_x(opcode)]);
        if is_pressed {
            return self.pc + 4;
        }
        self.pc + 2
    }

    /**
     * Skip next instruction if the key/w value of Vx is not pressed.
     * If the key is in the up position; incr PC by 2.
     * Otherwise, incr PC by 4.
     */
    fn handle_exa1(&mut self, opcode: u16) -> u16 {
        let is_pressed = self.keyboard.is_key_pressed(self.reg_v[get_x(opcode)]);
        if is_pressed {
            return self.pc + 2;
        }
        self.pc + 4
    }

    /**
     * Set Vx to equal delay timers value
     */
    fn handle_fx07(&mut self, opcode: u16) -> u16 {
        self.reg_v[get_x(opcode)] = self.delay_timer;
        self.pc + 2
    }
    /**
     * Wait for a key press, store the value of the key in Vx.
     * All execution stops until a key is pressed, then the value of that key is stored in Vx.
     */
    fn handle_fx0a(&mut self, opcode: u16) -> u16 {
        self.keyboard_blocking = true;
        self.reg_v[get_x(opcode)] = self.keycode;
        self.pc + 2
    }

    /**
     * Set delay timer to equal vx.
     */
    fn handle_fx15(&mut self, opcode: u16) -> u16 {
        self.delay_timer = self.reg_v[get_x(opcode)];
        self.pc + 2
    }
    /**
     * Set sound timer to equal vx.
     */
    fn handle_fx18(&mut self, opcode: u16) -> u16 {
        self.sound_timer = self.reg_v[get_x(opcode)];
        self.pc + 2
    }

    /**
     * Set I = I + Vx.
     * The values of I and Vx are added, and the results are stored in I.
     */
    fn handle_fx1e(&mut self, opcode: u16) -> u16 {
        let i = self.reg_i + self.reg_v[get_x(opcode)] as usize;
        self.reg_i = i;
        self.pc + 2
    }

    /**
     *  Set I = location of sprite for digit Vx.
     *  Sprites are 5 bytes long
     */
    fn handle_fx29(&mut self, opcode: u16) -> u16 {
        self.reg_i = (self.reg_v[get_x(opcode)] as usize) * 5;
        self.pc + 2
    }

    /**
     * Store BCD representation of Vx in memory locations I, I+1, and I+2.
     * The interpreter takes the decimal value of Vx, and places the hundreds digit in memory at location in I,
     * the tens digit at location I+1, and the ones digit at location I+2.
     */
    fn handle_fx33(&mut self, opcode: u16) -> u16 {
        let v_x = self.reg_v[get_x(opcode)];
        self.memory[self.reg_i] = v_x / 100;
        self.memory[self.reg_i + 1] = (v_x % 100) / 10;
        self.memory[self.reg_i + 2] = v_x % 10;
        self.pc + 2
    }

    /**
     * Store registers V0 through Vx in memory starting at location I.
     * The interpreter copies the values of registers V0 through Vx into memory, starting at the address in I.
     **/
    fn handle_fx55(&mut self, opcode: u16) -> u16 {
        for x in 0..get_x(opcode) + 1 {
            self.memory[self.reg_i + x] = self.reg_v[x];
        }
        self.pc + 2
    }
    /**
     * Read registers V0 through Vx from memory starting at location I.
     * The interpreter reads values from memory starting at location I into registers V0 through Vx.
     * Read values from memory into reg_v0 to reg_vx
     */
    fn handle_fx65(&mut self, opcode: u16) -> u16 {
        for x in 0..get_x(opcode) + 1 {
            self.reg_v[x] = self.memory[self.reg_i + x];
        }
        self.pc + 2
    }
}

fn get_nnn(opcode: u16) -> usize {
    (opcode & 0x0fff) as usize
}
/**
 * Returns the integer representation of x from the u16.
 */
fn get_x(opcode: u16) -> usize {
    ((opcode & 0x0f00) >> 8) as usize
}
/**
 * Returns the integer representation of kk from the u16.
 */
fn get_kk(opcode: u16) -> usize {
    (opcode & 0x00ff) as usize
}

fn get_y(opcode: u16) -> usize {
    ((opcode & 0x00f0) >> 4) as usize
}
fn get_n(opcode: u16) -> usize {
    (opcode & 0x000f) as usize
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_handle_2nnn() {
        let mut cpu = CPU::new();
        let opcode = 0x2111;
        cpu.handle_opcode(opcode);
        assert_eq!(cpu.pc, 0x111 as u16);
    }
    #[test]
    fn test_handle_3xkk() {
        let mut cpu = CPU::new();
        let opcode = 0x3122;
        cpu.handle_opcode(opcode);
        assert_eq!(cpu.pc, 2 as u16);
    }
    #[test]
    fn test_handle_4xkk() {
        let mut cpu = CPU::new();
        let opcode = 0x4122;
        cpu.handle_opcode(opcode);
        assert_eq!(cpu.pc, 4 as u16);
    }
    #[test]
    fn test_handle_5xy0() {
        let mut cpu = CPU::new();
        let opcode = 0x5000;
        cpu.handle_opcode(opcode);
        assert_eq!(cpu.pc, 4 as u16);
    }
    #[test]
    fn test_handle_6xkk() {
        let mut cpu = CPU::new();
        let opcode = 0x6666;
        cpu.handle_opcode(opcode);
        assert_eq!(cpu.reg_v[6], get_kk(opcode) as u8);
    }
    #[test]
    fn test_handle_7xkk() {
        let mut cpu = CPU::new();
        let opcode = 0x7777;
        cpu.reg_v[7] = 0x1;
        cpu.handle_opcode(opcode);
        assert_eq!(cpu.reg_v[7], get_kk(opcode) as u8 + 0x1);
    }
    #[test]
    fn test_handle_8xy0() {
        let mut cpu = CPU::new();
        let opcode = 0x8120;
        cpu.reg_v[2] = 0x5;
        cpu.handle_opcode(opcode);
        assert_eq!(cpu.reg_v[1], 0x5);
    }
    #[test]
    fn test_handle_8xy1() {
        let mut cpu = CPU::new();
        let opcode = 0x8121;
        cpu.reg_v[1] = 0x11;
        cpu.reg_v[2] = 0x10;
        cpu.handle_opcode(opcode);
        assert_eq!(cpu.reg_v[1], 0x11);
    }
    #[test]
    fn test_handle_8xy2() {
        let mut cpu = CPU::new();
        let opcode = 0x8122;
        cpu.reg_v[1] = 0x11;
        cpu.reg_v[2] = 0x10;
        cpu.handle_opcode(opcode);
        assert_eq!(cpu.reg_v[1], 0x10);
    }
    #[test]
    fn test_handle_8xy3() {
        let mut cpu = CPU::new();
        let opcode = 0x8123;
        cpu.reg_v[1] = 0x11;
        cpu.reg_v[2] = 0x10;
        cpu.handle_opcode(opcode);
        assert_eq!(cpu.reg_v[1], 0x01);
    }
    #[test]
    fn test_handle_8xy4() {
        let mut cpu = CPU::new();
        let opcode = 0x8123;
        cpu.reg_v[1] = 0x11;
        cpu.reg_v[2] = 0x10;
        cpu.handle_opcode(opcode);
        assert_eq!(cpu.reg_v[1], 0x01);
    }
    #[test]
    fn test_handle_8xy5() {
        let mut cpu = CPU::new();
        let opcode = 0x8125;
        cpu.reg_v[1] = 0x11;
        cpu.reg_v[2] = 0x10;
        cpu.handle_opcode(opcode);
        assert_eq!(cpu.reg_v[1], (0x11 as u8).wrapping_sub(0x10 as u8));
        assert_eq!(cpu.reg_v[0x0f], 1);
        assert_eq!(cpu.pc, 2);
    }

    // need to fix
    #[test]
    fn test_handle_8xy6() {
        let mut cpu = CPU::new();
        let opcode = 0x8126;
        cpu.reg_v[1] = 0x10;
        cpu.handle_opcode(opcode);
        assert_eq!(cpu.reg_v[1], 0x10 >> 1);
        assert_eq!(cpu.reg_v[0x0f], 0);
        cpu = CPU::new();
        cpu.reg_v[1] = 0x01;
        cpu.handle_opcode(opcode);
        assert_eq!(cpu.reg_v[0x0f], 1);
        assert_eq!(cpu.pc, 2);
    }

    #[test]
    fn test_handle_8xye() {
        let mut cpu = CPU::new();
        let opcode = 0x812e;
        cpu.reg_v[1] = 0x11;
        cpu.handle_opcode(opcode);
        assert_eq!(cpu.reg_v[1], 0x11 << 1);
        assert_eq!(cpu.reg_v[0x0f], 0);
        assert_eq!(cpu.pc, 2);
        cpu = CPU::new();
        let opcode = 0x812e;
        cpu.reg_v[1] = 0b111111;
        cpu.handle_opcode(opcode);
    }

    #[test]
    fn test_handle_9xy0() {
        let mut cpu = CPU::new();
        let opcode = 0x9120;
        cpu.reg_v[1] = 0x10;
        cpu.reg_v[2] = 0x33;
        cpu.handle_opcode(opcode);
        assert_eq!(cpu.pc, 4);
        cpu = CPU::new();
        cpu.reg_v[1] = 0x10;
        cpu.reg_v[2] = 0x10;
        cpu.handle_opcode(opcode);
        assert_eq!(cpu.pc, 2);
    }
    #[test]
    fn test_handle_annn() {
        let mut cpu = CPU::new();
        let opcode = 0xa111;
        cpu.handle_opcode(opcode);
        assert_eq!(cpu.pc, 2);
        assert_eq!(cpu.reg_i, 0x111);
    }
    #[test]
    fn test_handle_bnnn() {
        let mut cpu = CPU::new();
        let opcode = 0xb111;
        cpu.reg_v[0] = 0x11;
        cpu.handle_opcode(opcode);
        let result = (0x11 as usize) + (0x111 & 0xfff) as usize;
        assert_eq!(cpu.pc, result as u16);
    }
    #[test]
    fn test_handle_cxkk() {
        let mut cpu = CPU::new();
        let opcode = 0xc100;
        cpu.reg_v[1] = 0x11;
        cpu.handle_opcode(opcode);
        assert_eq!(cpu.reg_v[1], 0);
        cpu = CPU::new();
        let opcode = 0xc111;
        cpu.reg_v[1] = 0x11;
        cpu.handle_opcode(opcode);
        assert_ne!(cpu.reg_v[1], 0);
    }
    #[test]
    fn test_handle_dxyn() {
        let mut cpu = CPU::new();
        let opcode = 0xd120;
        cpu.handle_opcode(opcode);
        assert_eq!(cpu.display_changed, true);
    }
    #[test]
    fn test_handle_ex9e() {
        let mut cpu = CPU::new();
        let opcode = 0xe19e;
        cpu.handle_opcode(opcode);
        assert_eq!(cpu.pc, 2);
    }

    #[test]
    fn test_handle_fx07() {
        let mut cpu = CPU::new();
        let opcode = 0xf107;
        cpu.delay_timer = 0x2;
        cpu.reg_v[1] = 0x1;
        cpu.handle_opcode(opcode);
        assert_eq!(cpu.delay_timer, 0x2);
    }
    #[test]
    fn test_handle_fx15() {
        let mut cpu = CPU::new();
        let opcode = 0xf115;
        cpu.reg_v[1] = 0x5;
        cpu.handle_opcode(opcode);
        assert_eq!(cpu.delay_timer, 0x5);
        assert_eq!(cpu.pc, 2);
    }
    #[test]
    fn test_handle_fx18() {
        let mut cpu = CPU::new();
        let opcode = 0xf118;
        cpu.reg_v[1] = 0x5;
        cpu.handle_opcode(opcode);
        assert_eq!(cpu.sound_timer, 0x5);
        assert_eq!(cpu.pc, 2);
    }
    #[test]
    fn test_handle_fx1e() {
        let mut cpu = CPU::new();
        let opcode = 0xf11e;
        cpu.reg_v[1] = 0x5;
        cpu.reg_i = 0x1;
        cpu.handle_opcode(opcode);
        assert_eq!(cpu.reg_i, 0x6);
        assert_eq!(cpu.pc, 2);
    }
    #[test]
    fn test_handle_fx29() {
        let mut cpu = CPU::new();
        let opcode = 0xf129;
        cpu.reg_v[1] = 5;
        cpu.handle_opcode(opcode);
        assert_eq!(cpu.reg_i, 25);
    }
    #[test]
    fn test_handle_fx33() {
        let mut cpu = CPU::new();
        let opcode = 0xf133;
        cpu.reg_v[1] = 245;
        cpu.reg_i = 0;
        cpu.handle_opcode(opcode);
        assert_eq!(cpu.memory[0], 2);
        assert_eq!(cpu.memory[1], 4);
        assert_eq!(cpu.memory[2], 5);
    }
    #[test]
    fn test_handle_fx55() {
        let mut cpu = CPU::new();
        let opcode = 0xf355;
        cpu.reg_i = 0x300;
        cpu.reg_v[0] = 1;
        cpu.reg_v[1] = 2;
        cpu.reg_v[2] = 3;
        cpu.handle_opcode(opcode);
        assert_eq!(cpu.memory[0x300], 1);
        assert_eq!(cpu.memory[0x301], 2);
        assert_eq!(cpu.memory[0x302], 3);
    }
    #[test]
    fn test_handle_fx65() {
        let mut cpu = CPU::new();
        let opcode = 0xf165;
        cpu.reg_i = 0x200;
        cpu.memory[0x200] = 2;
        cpu.memory[0x201] = 3;
        cpu.handle_opcode(opcode);
        assert_eq!(cpu.reg_v[0], 2);
        assert_eq!(cpu.reg_v[1], 3);
    }
}
