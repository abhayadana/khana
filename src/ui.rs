//! SDL rendering for Khaṇa's two MVP screens.

use crate::music::chord::Chord;
use crate::music::recommendation::{HarmonicDirection, RecommendationClass, RecommendationSet};
use crate::phrase::{Phrase, RecorderStatus};
use crate::voice::{Voice, VoiceScope};
use sdl2::pixels::Color;
use sdl2::rect::Rect;

/// Stateless 1024x768 renderer.
pub struct Ui {
    width: u32,
}

impl Ui {
    /// Creates the UI renderer.
    pub const fn new(width: u32, _height: u32) -> Self {
        Self { width }
    }

    /// Draws the Performance screen.
    #[allow(clippy::too_many_arguments)]
    pub fn draw_performance(
        &self,
        canvas: &mut sdl2::render::Canvas<sdl2::video::Window>,
        current_chord: Chord,
        chord_elapsed_beats: f64,
        recommendations: &RecommendationSet,
        selected_class: RecommendationClass,
        selected_row: usize,
        direction: HarmonicDirection,
        voices: &[Voice; 4],
        scope: VoiceScope,
        recorder_status: RecorderStatus,
        next_recording_armed: bool,
        quantize_beats: f64,
        phrase_count: usize,
    ) -> Result<(), String> {
        canvas.set_draw_color(background());
        canvas.clear();

        draw_text(canvas, 36, 24, "KHANA", 5, text_color())?;
        draw_text(canvas, 210, 28, "C MAJOR", 3, dim_text())?;
        draw_text(canvas, 410, 28, "120 BPM", 3, dim_text())?;
        draw_text(canvas, 600, 28, "4/4", 3, dim_text())?;

        draw_panel(canvas, Rect::new(32, 82, 250, 220), panel())?;
        draw_text(canvas, 54, 100, "CURRENT", 2, dim_text())?;
        draw_text(canvas, 54, 145, &current_chord.symbol(), 7, text_color())?;
        draw_text(
            canvas,
            54,
            235,
            &format!("{:.1} BEATS", chord_elapsed_beats),
            2,
            accent(),
        )?;

        draw_text(canvas, 315, 88, "NEXT", 2, dim_text())?;
        draw_recommendations(canvas, recommendations, selected_class, selected_row)?;

        draw_panel(canvas, Rect::new(780, 82, 212, 220), panel())?;
        draw_text(canvas, 800, 100, "DIRECTION", 2, dim_text())?;
        draw_text(canvas, 800, 138, direction.label(), 3, accent())?;
        draw_text(canvas, 800, 194, "SCOPE", 2, dim_text())?;
        draw_text(canvas, 800, 230, &scope.label(), 4, text_color())?;

        draw_voice_rows(canvas, voices, scope)?;

        draw_panel(canvas, Rect::new(32, 660, 960, 82), panel())?;
        draw_recording_strip(
            canvas,
            recorder_status,
            next_recording_armed,
            quantize_beats,
            phrase_count,
        )?;

        Ok(())
    }

    /// Draws the Phrase Editor screen.
    #[allow(clippy::too_many_arguments)]
    pub fn draw_phrase_editor(
        &self,
        canvas: &mut sdl2::render::Canvas<sdl2::video::Window>,
        phrase: Option<&Phrase>,
        phrase_index: usize,
        phrase_count: usize,
        selected_event: usize,
        pixels_per_beat: f32,
    ) -> Result<(), String> {
        canvas.set_draw_color(background());
        canvas.clear();

        draw_text(canvas, 36, 24, "KHANA", 5, text_color())?;
        draw_text(canvas, 210, 28, "PHRASE EDITOR", 3, accent())?;
        draw_text(canvas, 760, 28, "TAB PERFORMANCE", 2, dim_text())?;

        let Some(phrase) = phrase else {
            draw_text(canvas, 360, 310, "NO PHRASES YET", 4, dim_text())?;
            draw_text(
                canvas,
                270,
                370,
                "RECORD ON PERFORMANCE SCREEN",
                2,
                dim_text(),
            )?;
            return Ok(());
        };

        let previous = if phrase_index > 0 {
            format!("P{:02}", phrase_index)
        } else {
            "--".to_owned()
        };
        let current = format!("P{:02}", phrase.id);
        let next = if phrase_index + 1 < phrase_count {
            format!("P{:02}", phrase_index + 2)
        } else {
            "--".to_owned()
        };

        draw_text(canvas, 42, 92, &previous, 3, dim_text())?;
        draw_text(canvas, 130, 92, &format!("[{}]", current), 3, text_color())?;
        draw_text(canvas, 260, 92, &next, 3, dim_text())?;
        draw_text(
            canvas,
            720,
            92,
            &format!("{:.1} BEATS", phrase.duration_beats()),
            3,
            dim_text(),
        )?;

        draw_timeline(canvas, phrase, selected_event, pixels_per_beat, self.width)?;

        if let Some(event) = phrase.events.get(selected_event) {
            draw_panel(canvas, Rect::new(32, 610, 960, 112), panel())?;
            draw_text(canvas, 52, 626, "SELECTED", 2, dim_text())?;
            draw_text(canvas, 52, 660, &event.chord.symbol(), 5, text_color())?;
            draw_text(
                canvas,
                260,
                650,
                &format!("START {:.1}", event.start_beat),
                2,
                dim_text(),
            )?;
            draw_text(
                canvas,
                260,
                680,
                &format!("LENGTH {:.1}", event.duration_beats),
                2,
                dim_text(),
            )?;
            draw_text(
                canvas,
                550,
                642,
                "Z CHANGE  D DUP  BACKSPACE DELETE",
                2,
                accent(),
            )?;
            draw_text(
                canvas,
                550,
                680,
                "F SHORTER  R LONGER  -/+ ZOOM",
                2,
                dim_text(),
            )?;
        }

        Ok(())
    }
}

fn draw_recommendations(
    canvas: &mut sdl2::render::Canvas<sdl2::video::Window>,
    recommendations: &RecommendationSet,
    selected_class: RecommendationClass,
    selected_row: usize,
) -> Result<(), String> {
    let start_x = 310_i32;
    let width = 144_u32;
    let gap = 12_i32;

    for class in RecommendationClass::ALL {
        let column = class.index() as i32;
        let x = start_x + column * (width as i32 + gap);
        let color = class_color(class);

        draw_text(canvas, x + 10, 112, class.label(), 2, color)?;

        for row in 0..3 {
            let y = 146 + row as i32 * 50;
            let selected = class == selected_class && row == selected_row;
            draw_panel(
                canvas,
                Rect::new(x, y, width, 40),
                if selected { color } else { panel() },
            )?;
            let chord = recommendations.get(class, row);
            draw_text(
                canvas,
                x + 12,
                y + 10,
                &chord.symbol(),
                2,
                if selected { background() } else { text_color() },
            )?;
        }
    }

    Ok(())
}

fn draw_voice_rows(
    canvas: &mut sdl2::render::Canvas<sdl2::video::Window>,
    voices: &[Voice; 4],
    scope: VoiceScope,
) -> Result<(), String> {
    draw_text(canvas, 36, 330, "VOICES", 2, dim_text())?;
    draw_text(canvas, 260, 330, "DENS", 2, dim_text())?;
    draw_text(canvas, 430, 330, "DYN", 2, dim_text())?;
    draw_text(canvas, 600, 330, "SPRD", 2, dim_text())?;
    draw_text(canvas, 780, 330, "COLOR", 2, dim_text())?;

    for (index, voice) in voices.iter().enumerate() {
        let y = 370 + index as i32 * 66;
        let scoped = scope == VoiceScope::All || scope == VoiceScope::One(index);

        draw_panel(
            canvas,
            Rect::new(32, y - 8, 960, 56),
            if scoped { scoped_panel() } else { panel() },
        )?;
        draw_text(
            canvas,
            52,
            y + 6,
            &format!("{} {}", index + 1, voice.role.label()),
            2,
            text_color(),
        )?;
        draw_value_bar(canvas, 250, y + 4, voice.parameters.density)?;
        draw_value_bar(canvas, 420, y + 4, voice.parameters.dynamics)?;
        draw_value_bar(canvas, 590, y + 4, voice.parameters.spread)?;
        draw_value_bar(canvas, 770, y + 4, voice.parameters.color)?;
    }

    Ok(())
}

fn draw_recording_strip(
    canvas: &mut sdl2::render::Canvas<sdl2::video::Window>,
    status: RecorderStatus,
    next_armed: bool,
    quantize_beats: f64,
    phrase_count: usize,
) -> Result<(), String> {
    let status_text = match status {
        RecorderStatus::Idle => "IDLE".to_owned(),
        RecorderStatus::Armed { start_beat } => {
            format!("ARMED START {:.1}", start_beat)
        }
        RecorderStatus::Recording { phrase_id } => {
            format!("RECORDING P{:02}", phrase_id)
        }
        RecorderStatus::Stopping {
            phrase_id,
            stop_beat,
        } => format!("P{:02} STOP {:.1}", phrase_id, stop_beat),
    };

    draw_text(canvas, 52, 678, "CAPTURE", 2, dim_text())?;
    draw_text(canvas, 180, 678, &status_text, 3, accent())?;
    draw_text(
        canvas,
        610,
        678,
        &format!("Q {:.0} BEATS", quantize_beats),
        2,
        dim_text(),
    )?;
    draw_text(
        canvas,
        805,
        678,
        &format!("SAVED {}", phrase_count),
        2,
        dim_text(),
    )?;

    if next_armed {
        draw_text(canvas, 180, 712, "NEXT PHRASE ARMED", 2, text_color())?;
    } else {
        draw_text(canvas, 180, 712, "P RECORD   N ARM NEXT", 2, dim_text())?;
    }

    Ok(())
}

fn draw_timeline(
    canvas: &mut sdl2::render::Canvas<sdl2::video::Window>,
    phrase: &Phrase,
    selected_event: usize,
    pixels_per_beat: f32,
    screen_width: u32,
) -> Result<(), String> {
    let left = 44_f32;
    let right = screen_width as f32 - 44.0;
    let visible_width = right - left;

    let selected_start = phrase
        .events
        .get(selected_event)
        .map(|event| event.start_beat as f32)
        .unwrap_or(0.0);

    let visible_beats = visible_width / pixels_per_beat;
    let view_start = (selected_start - visible_beats * 0.35).max(0.0);
    let view_end = view_start + visible_beats;

    draw_text(canvas, 44, 160, "CHORD TIMELINE", 2, dim_text())?;
    canvas.set_draw_color(grid_line());
    canvas.draw_line((44, 520), (screen_width as i32 - 44, 520))?;

    for (index, event) in phrase.events.iter().enumerate() {
        let start = event.start_beat as f32;
        let end = start + event.duration_beats as f32;

        if end < view_start || start > view_end {
            continue;
        }

        let x = left + (start - view_start) * pixels_per_beat;
        let width = (event.duration_beats as f32 * pixels_per_beat).max(18.0);
        let selected = index == selected_event;

        draw_panel(
            canvas,
            Rect::new(x as i32, 245, width as u32, 175),
            if selected {
                timeline_selected()
            } else {
                timeline_block()
            },
        )?;
        draw_text(
            canvas,
            x as i32 + 10,
            275,
            &event.chord.symbol(),
            3,
            if selected { background() } else { text_color() },
        )?;
        draw_text(
            canvas,
            x as i32 + 10,
            370,
            &format!("{:.1}", event.duration_beats),
            2,
            if selected { background() } else { dim_text() },
        )?;
    }

    Ok(())
}

fn draw_value_bar(
    canvas: &mut sdl2::render::Canvas<sdl2::video::Window>,
    x: i32,
    y: i32,
    value: f32,
) -> Result<(), String> {
    let width = 125_u32;
    draw_panel(canvas, Rect::new(x, y, width, 18), dim_panel())?;
    draw_panel(
        canvas,
        Rect::new(x, y, (width as f32 * value.clamp(0.0, 1.0)) as u32, 18),
        accent(),
    )?;
    draw_text(
        canvas,
        x + 42,
        y + 27,
        &format!("{:.2}", value),
        1,
        dim_text(),
    )?;
    Ok(())
}

fn draw_panel(
    canvas: &mut sdl2::render::Canvas<sdl2::video::Window>,
    rect: Rect,
    color: Color,
) -> Result<(), String> {
    canvas.set_draw_color(color);
    canvas.fill_rect(rect)
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
                    x + column * scale as i32,
                    y + row as i32 * scale as i32,
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
        'A' => Some([14, 17, 17, 31, 17, 17, 17]),
        'B' => Some([30, 17, 17, 30, 17, 17, 30]),
        'C' => Some([14, 17, 16, 16, 16, 17, 14]),
        'D' => Some([30, 17, 17, 17, 17, 17, 30]),
        'E' => Some([31, 16, 16, 30, 16, 16, 31]),
        'F' => Some([31, 16, 16, 30, 16, 16, 16]),
        'G' => Some([14, 17, 16, 23, 17, 17, 14]),
        'H' => Some([17, 17, 17, 31, 17, 17, 17]),
        'I' => Some([31, 4, 4, 4, 4, 4, 31]),
        'J' => Some([7, 2, 2, 2, 18, 18, 12]),
        'K' => Some([17, 18, 20, 24, 20, 18, 17]),
        'L' => Some([16, 16, 16, 16, 16, 16, 31]),
        'M' => Some([17, 27, 21, 21, 17, 17, 17]),
        'N' => Some([17, 25, 21, 19, 17, 17, 17]),
        'O' => Some([14, 17, 17, 17, 17, 17, 14]),
        'P' => Some([30, 17, 17, 30, 16, 16, 16]),
        'Q' => Some([14, 17, 17, 17, 21, 18, 13]),
        'R' => Some([30, 17, 17, 30, 20, 18, 17]),
        'S' => Some([15, 16, 16, 14, 1, 1, 30]),
        'T' => Some([31, 4, 4, 4, 4, 4, 4]),
        'U' => Some([17, 17, 17, 17, 17, 17, 14]),
        'V' => Some([17, 17, 17, 17, 17, 10, 4]),
        'W' => Some([17, 17, 17, 21, 21, 21, 10]),
        'X' => Some([17, 17, 10, 4, 10, 17, 17]),
        'Y' => Some([17, 17, 10, 4, 4, 4, 4]),
        'Z' => Some([31, 1, 2, 4, 8, 16, 31]),
        '0' => Some([14, 17, 19, 21, 25, 17, 14]),
        '1' => Some([4, 12, 4, 4, 4, 4, 14]),
        '2' => Some([14, 17, 1, 2, 4, 8, 31]),
        '3' => Some([30, 1, 1, 14, 1, 1, 30]),
        '4' => Some([2, 6, 10, 18, 31, 2, 2]),
        '5' => Some([31, 16, 16, 30, 1, 1, 30]),
        '6' => Some([14, 16, 16, 30, 17, 17, 14]),
        '7' => Some([31, 1, 2, 4, 8, 8, 8]),
        '8' => Some([14, 17, 17, 14, 17, 17, 14]),
        '9' => Some([14, 17, 17, 15, 1, 1, 14]),
        '#' => Some([10, 31, 10, 10, 31, 10, 0]),
        '[' => Some([14, 8, 8, 8, 8, 8, 14]),
        ']' => Some([14, 2, 2, 2, 2, 2, 14]),
        '-' => Some([0, 0, 0, 31, 0, 0, 0]),
        '/' => Some([1, 2, 4, 8, 16, 0, 0]),
        '.' => Some([0, 0, 0, 0, 0, 12, 12]),
        '+' => Some([0, 4, 4, 31, 4, 4, 0]),
        _ => None,
    }
}

fn background() -> Color {
    Color::RGB(16, 18, 24)
}

fn panel() -> Color {
    Color::RGB(33, 38, 49)
}

fn scoped_panel() -> Color {
    Color::RGB(43, 50, 64)
}

fn dim_panel() -> Color {
    Color::RGB(56, 63, 77)
}

fn text_color() -> Color {
    Color::RGB(229, 233, 240)
}

fn dim_text() -> Color {
    Color::RGB(130, 141, 158)
}

fn accent() -> Color {
    Color::RGB(78, 220, 205)
}

fn class_color(class: RecommendationClass) -> Color {
    match class {
        RecommendationClass::Safe => Color::RGB(76, 219, 137),
        RecommendationClass::Colorful => Color::RGB(240, 189, 74),
        RecommendationClass::Bold => Color::RGB(223, 91, 189),
    }
}

fn timeline_block() -> Color {
    Color::RGB(43, 54, 72)
}

fn timeline_selected() -> Color {
    Color::RGB(78, 220, 205)
}

fn grid_line() -> Color {
    Color::RGB(70, 78, 93)
}
