mod input;
mod midi;
mod music;
mod ui;

use std::collections::HashSet;
use std::thread;
use std::time::Duration;

use input::{InputAction, InputEvent, InputMapper};
use midi::MidiEngine;
use music::MusicState;
use sdl2::event::Event;
use sdl2::keyboard::Keycode;
use ui::Ui;

const WINDOW_WIDTH: u32 = 1024;
const WINDOW_HEIGHT: u32 = 768;

fn main() -> Result<(), String> {
    let sdl_context = sdl2::init()?;
    let video = sdl_context.video()?;

    let window = video
        .window(
            "Khaṇa — arrows select degree, Z plays, Esc quits",
            WINDOW_WIDTH,
            WINDOW_HEIGHT,
        )
        .position_centered()
        .build()
        .map_err(|error| error.to_string())?;

    let mut canvas = window
        .into_canvas()
        .present_vsync()
        .build()
        .map_err(|error| error.to_string())?;

    let mut event_pump = sdl_context.event_pump()?;

    let input_mapper = InputMapper::new();
    let mut music_state = MusicState::new_c_major();
    let mut midi_engine = MidiEngine::new_virtual("Khana MIDI")?;
    let ui = Ui::new(WINDOW_WIDTH, WINDOW_HEIGHT);

    let mut held_actions: HashSet<InputAction> = HashSet::new();
    let mut active_note: Option<u8> = None;

    println!("Khaṇa v0.2.0");
    println!("Virtual MIDI output: Khana MIDI");
    println!("Left/Right: select degree");
    println!("Z: play selected degree");
    println!("Esc: quit");

    'running: loop {
        for event in event_pump.poll_iter() {
            match event {
                Event::Quit { .. }
                | Event::KeyDown {
                    keycode: Some(Keycode::Escape),
                    ..
                } => break 'running,
                Event::KeyDown {
                    keycode: Some(keycode),
                    repeat: false,
                    ..
                } => {
                    if let Some(input_event) = input_mapper.key_down(keycode) {
                        handle_input_event(
                            input_event,
                            &mut held_actions,
                            &mut music_state,
                            &mut midi_engine,
                            &mut active_note,
                        )?;
                    }
                }
                Event::KeyUp {
                    keycode: Some(keycode),
                    repeat: false,
                    ..
                } => {
                    if let Some(input_event) = input_mapper.key_up(keycode) {
                        handle_input_event(
                            input_event,
                            &mut held_actions,
                            &mut music_state,
                            &mut midi_engine,
                            &mut active_note,
                        )?;
                    }
                }
                _ => {}
            }
        }

        ui.draw(&mut canvas, &music_state, &held_actions)?;
        canvas.present();

        // Avoid a busy loop on systems where VSync is unavailable.
        thread::sleep(Duration::from_millis(1));
    }

    if let Some(note) = active_note {
        let _ = midi_engine.note_off(note);
    }

    println!("Khaṇa exited cleanly.");
    Ok(())
}

fn handle_input_event(
    input_event: InputEvent,
    held_actions: &mut HashSet<InputAction>,
    music_state: &mut MusicState,
    midi_engine: &mut MidiEngine,
    active_note: &mut Option<u8>,
) -> Result<(), String> {
    match input_event {
        InputEvent::Pressed(action) => {
            held_actions.insert(action);

            match action {
                InputAction::SelectPrevious => {
                    music_state.select_previous_degree();
                    print_selection(music_state);
                }
                InputAction::SelectNext => {
                    music_state.select_next_degree();
                    print_selection(music_state);
                }
                InputAction::Play => {
                    // Only one prototype voice for now. This keeps note-off behavior obvious.
                    if active_note.is_none() {
                        let note = music_state.selected_midi_note();
                        midi_engine.note_on(note, music_state.velocity())?;
                        *active_note = Some(note);

                        println!(
                            "NOTE ON  degree={} note={} ({}) velocity={}",
                            music_state.selected_degree(),
                            note,
                            music_state.selected_note_name(),
                            music_state.velocity()
                        );
                    }
                }
            }
        }
        InputEvent::Released(action) => {
            held_actions.remove(&action);

            if action == InputAction::Play {
                if let Some(note) = active_note.take() {
                    midi_engine.note_off(note)?;
                    println!("NOTE OFF note={note}");
                }
            }
        }
    }

    Ok(())
}

fn print_selection(music_state: &MusicState) {
    println!(
        "Selected degree {} -> {} (MIDI {})",
        music_state.selected_degree(),
        music_state.selected_note_name(),
        music_state.selected_midi_note()
    );
}
