use std::collections::HashSet;

use crate::input::InputAction;
use crate::music::MusicState;
use sdl2::pixels::Color;
use sdl2::rect::Rect;

fn background() -> Color {
    Color::RGB(18, 18, 22)
}

fn panel() -> Color {
    Color::RGB(44, 44, 52)
}

fn selected_color() -> Color {
    Color::RGB(225, 225, 225)
}

fn active() -> Color {
    Color::RGB(150, 150, 160)
}

fn text_color() -> Color {
    Color::RGB(220, 220, 220)
}

fn dim_text() -> Color {
    Color::RGB(125, 125, 135)
}

pub struct Ui {
    width: u32,
    height: u32,
}

impl Ui {
    pub const fn new(width: u32, height: u32) -> Self {
        Self { width, height }
    }

    pub fn draw(
        &self,
        canvas: &mut sdl2::render::Canvas<sdl2::video::Window>,
        music_state: &MusicState,
        held_actions: &HashSet<InputAction>,
    ) -> Result<(), String> {
        canvas.set_draw_color(background());
        canvas.clear();

        self.draw_title(canvas)?;
        self.draw_scale(canvas, music_state)?;
        self.draw_status(canvas, music_state)?;
        self.draw_controls(canvas, held_actions)?;

        Ok(())
    }

    fn draw_title(
        &self,
        canvas: &mut sdl2::render::Canvas<sdl2::video::Window>,
    ) -> Result<(), String> {
        draw_text(canvas, 410, 75, "KHANA", 6, text_color())?;
        draw_text(canvas, 433, 135, "C MAJOR", 3, dim_text())?;
        Ok(())
    }

    fn draw_scale(
        &self,
        canvas: &mut sdl2::render::Canvas<sdl2::video::Window>,
        music_state: &MusicState,
    ) -> Result<(), String> {
        let tile_width: i32 = 100;
        let gap: i32 = 20;
        let total_width: i32 = (tile_width * 7) + (gap * 6);
        let start_x: i32 = (self.width as i32 - total_width) / 2;
        let y: i32 = 285;

        for index in 0..7 {
            let x = start_x + (index as i32 * (tile_width + gap));
            let is_selected = index + 1 == music_state.selected_degree();

            canvas.set_draw_color(if is_selected { selected_color() } else { panel() });
            canvas.fill_rect(Rect::new(x, y, tile_width as u32, 110))?;

            let digit_color = if is_selected { background() } else { text_color() };
            draw_text(
                canvas,
                x + 37,
                y + 30,
                &(index + 1).to_string(),
                7,
                digit_color,
            )?;
        }

        Ok(())
    }

    fn draw_status(
        &self,
        canvas: &mut sdl2::render::Canvas<sdl2::video::Window>,
        music_state: &MusicState,
    ) -> Result<(), String> {
        let degree_text = format!("DEGREE {}", music_state.selected_degree());
        let note_text = format!("NOTE {}", music_state.selected_note_name());

        draw_text(canvas, 372, 470, &degree_text, 4, text_color())?;
        draw_text(canvas, 390, 525, &note_text, 4, text_color())?;

        Ok(())
    }

    fn draw_controls(
        &self,
        canvas: &mut sdl2::render::Canvas<sdl2::video::Window>,
        held_actions: &HashSet<InputAction>,
    ) -> Result<(), String> {
        let base_y = self.height as i32 - 105;

        draw_text(canvas, 210, base_y, "LEFT RIGHT SELECT", 2, dim_text())?;

        let play_color = if held_actions.contains(&InputAction::Play) {
            active()
        } else {
            dim_text()
        };
        draw_text(canvas, 650, base_y, "Z PLAY", 2, play_color)?;

        Ok(())
    }
}

fn draw_text(
    canvas: &mut sdl2::render::Canvas<sdl2::video::Window>,
    x: i32,
    y: i32,
    text: &str,
    scale: u32,
    color: Color,
) -> Result<(), String> {
    let mut cursor_x = x;

    for character in text.chars() {
        if character == ' ' {
            cursor_x += (4 * scale) as i32;
            continue;
        }

        if let Some(pattern) = glyph(character) {
            draw_glyph(canvas, cursor_x, y, pattern, scale, color)?;
        }

        cursor_x += (6 * scale) as i32;
    }

    Ok(())
}

fn draw_glyph(
    canvas: &mut sdl2::render::Canvas<sdl2::video::Window>,
    x: i32,
    y: i32,
    pattern: [u8; 7],
    scale: u32,
    color: Color,
) -> Result<(), String> {
    canvas.set_draw_color(color);

    for (row, bits) in pattern.iter().enumerate() {
        for column in 0..5 {
            let mask = 1_u8 << (4 - column);

            if *bits & mask != 0 {
                canvas.fill_rect(Rect::new(
                    x + (column as i32 * scale as i32),
                    y + (row as i32 * scale as i32),
                    scale,
                    scale,
                ))?;
            }
        }
    }

    Ok(())
}

fn glyph(character: char) -> Option<[u8; 7]> {
    match character.to_ascii_uppercase() {
        'A' => Some([0b01110, 0b10001, 0b10001, 0b11111, 0b10001, 0b10001, 0b10001]),
        'C' => Some([0b01110, 0b10001, 0b10000, 0b10000, 0b10000, 0b10001, 0b01110]),
        'D' => Some([0b11110, 0b10001, 0b10001, 0b10001, 0b10001, 0b10001, 0b11110]),
        'E' => Some([0b11111, 0b10000, 0b10000, 0b11110, 0b10000, 0b10000, 0b11111]),
        'F' => Some([0b11111, 0b10000, 0b10000, 0b11110, 0b10000, 0b10000, 0b10000]),
        'G' => Some([0b01110, 0b10001, 0b10000, 0b10111, 0b10001, 0b10001, 0b01110]),
        'H' => Some([0b10001, 0b10001, 0b10001, 0b11111, 0b10001, 0b10001, 0b10001]),
        'I' => Some([0b11111, 0b00100, 0b00100, 0b00100, 0b00100, 0b00100, 0b11111]),
        'J' => Some([0b00111, 0b00010, 0b00010, 0b00010, 0b10010, 0b10010, 0b01100]),
        'K' => Some([0b10001, 0b10010, 0b10100, 0b11000, 0b10100, 0b10010, 0b10001]),
        'L' => Some([0b10000, 0b10000, 0b10000, 0b10000, 0b10000, 0b10000, 0b11111]),
        'M' => Some([0b10001, 0b11011, 0b10101, 0b10101, 0b10001, 0b10001, 0b10001]),
        'N' => Some([0b10001, 0b11001, 0b10101, 0b10011, 0b10001, 0b10001, 0b10001]),
        'O' => Some([0b01110, 0b10001, 0b10001, 0b10001, 0b10001, 0b10001, 0b01110]),
        'P' => Some([0b11110, 0b10001, 0b10001, 0b11110, 0b10000, 0b10000, 0b10000]),
        'R' => Some([0b11110, 0b10001, 0b10001, 0b11110, 0b10100, 0b10010, 0b10001]),
        'S' => Some([0b01111, 0b10000, 0b10000, 0b01110, 0b00001, 0b00001, 0b11110]),
        'T' => Some([0b11111, 0b00100, 0b00100, 0b00100, 0b00100, 0b00100, 0b00100]),
        'Y' => Some([0b10001, 0b10001, 0b01010, 0b00100, 0b00100, 0b00100, 0b00100]),
        'Z' => Some([0b11111, 0b00001, 0b00010, 0b00100, 0b01000, 0b10000, 0b11111]),
        '1' => Some([0b00100, 0b01100, 0b00100, 0b00100, 0b00100, 0b00100, 0b01110]),
        '2' => Some([0b01110, 0b10001, 0b00001, 0b00010, 0b00100, 0b01000, 0b11111]),
        '3' => Some([0b11110, 0b00001, 0b00001, 0b01110, 0b00001, 0b00001, 0b11110]),
        '4' => Some([0b00010, 0b00110, 0b01010, 0b10010, 0b11111, 0b00010, 0b00010]),
        '5' => Some([0b11111, 0b10000, 0b10000, 0b11110, 0b00001, 0b00001, 0b11110]),
        '6' => Some([0b01110, 0b10000, 0b10000, 0b11110, 0b10001, 0b10001, 0b01110]),
        '7' => Some([0b11111, 0b00001, 0b00010, 0b00100, 0b01000, 0b01000, 0b01000]),
        _ => None,
    }
}
