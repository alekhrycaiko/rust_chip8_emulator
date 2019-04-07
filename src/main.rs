extern crate sdl2;
use sdl2::event::Event;
use sdl2::keyboard::Keycode;
use std::env;
use std::fs::File;
use std::io::Read;

mod cpu;
mod display;
mod keyboard;

fn main() {
    let args: Vec<_> = env::args().collect();
    if args.len() < 1 {
        panic!()
    }
    let file_name = &args[1];
    let mut file = match File::open(file_name) {
        Ok(file) => file,
        Err(..) => panic!("file didnt exist"),
    };
    let mut buffer = Vec::new();
    file.read_to_end(&mut buffer).unwrap();
    let mut cpu = cpu::CPU::new(&buffer);
    let mut display = display::Display::new();
    let mut i = 0;
    let sdl_context = &display.sdl_context;
    let mut event_pump = sdl_context.event_pump().unwrap();
    'main_loop: loop {
        i = (i + 1) % 255;
        display.canvas.clear();
        for event in event_pump.poll_iter() {
            match event {
                Event::Quit { .. }
                | Event::KeyDown {
                    keycode: Some(Keycode::Escape),
                    ..
                } => break 'main_loop,
                _ => {}
            }
        }
        let result = cpu.cycle();
        println!("{}", result.flag);
        //        if result.flag == 1 {
        display.draw(result.display_memory);
        display.canvas.present();
        //      }
    }
}
