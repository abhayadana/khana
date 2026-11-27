//! SDL rendering for Khaṇa's two 1024×768 MVP screens.

use crate::music::chord::Chord;
use crate::music::recommendation::{HarmonicDirection, RecommendationClass, RecommendationSet};
use crate::music::settings::{MusicalSettings, SettingField};
use crate::music::voicing::{MAX_VOICING_NOTES, VoicingStyle};
use crate::performer::PerformerSnapshot;
use crate::phrase::{EditGrid, Phrase, RecorderStatus};
use crate::sync::{SyncMode, SyncStatus};
use crate::voice::{Voice, VoiceRender, VoiceScope};
use sdl2::pixels::Color;
use sdl2::rect::Rect;

/// All state needed to render the live Performance screen.
pub struct PerformanceView<'a> {
    pub timing: &'a str,
    pub beat_flash: Option<bool>,
    pub chord_gate: &'static str,
    pub bass_arrival: &'static str,
    pub groove: &'static str,
    pub texture_role: &'static str,
    pub panic_confirm: bool,
    pub macro_targets: [bool; 4],
    pub has_macro_group: bool,
    pub live_grid: &'static str,
    pub select_held: bool,
    pub voicing: VoicingStyle,
    pub pending_voicing: Option<VoicingStyle>,
    pub settings: MusicalSettings,
    pub setting_focus: Option<SettingField>,
    pub sync_status: SyncStatus,
    pub current_chord: Chord,
    pub chord_elapsed_beats: f64,
    pub current_render: VoiceRender,
    pub recommendations: &'a RecommendationSet,
    pub selected_class: RecommendationClass,
    pub selected_row: usize,
    pub direction: HarmonicDirection,
    pub voices: &'a [Voice; 4],
    pub performer_snapshots: [PerformerSnapshot; 4],
    pub scope: VoiceScope,
    pub recorder_status: RecorderStatus,
    pub next_recording_armed: bool,
    pub phrase_count: usize,
    pub midi_destination: &'a str,
    pub midi_clock: &'static str,
    pub midi_channels: [u8; 4],
    pub midi_transpose: [i8; 4],
    pub midi_input: Option<&'a str>,
}

/// All state needed to render the Phrase Editor screen.
pub struct PhraseEditorView<'a> {
    pub timing: &'a str,
    pub beat_flash: Option<bool>,
    pub settings: MusicalSettings,
    pub sync_status: SyncStatus,
    pub phrase: Option<&'a Phrase>,
    pub phrase_index: usize,
    pub phrase_count: usize,
    pub selected_event: usize,
    pub pixels_per_beat: f32,
    pub playing_phrase: Option<usize>,
    pub queued_phrase: Option<usize>,
    pub edit_grid: EditGrid,
    pub can_undo: bool,
    pub select_held: bool,
}

/// Stateless 1024×768 renderer.
pub struct Ui {
    width: u32,
    brick: bool,
}

impl Ui {
    pub const fn new(width: u32, brick: bool) -> Self {
        Self { width, brick }
    }

    pub fn draw_performance(
        &self,
        canvas: &mut sdl2::render::Canvas<sdl2::video::Window>,
        view: PerformanceView<'_>,
    ) -> Result<(), String> {
        let PerformanceView {
            timing,
            beat_flash,
            groove,
            macro_targets,
            has_macro_group,
            live_grid,
            select_held,
            voicing,
            pending_voicing,
            settings,
            setting_focus,
            sync_status,
            current_chord,
            chord_elapsed_beats,
            current_render,
            recommendations,
            selected_class,
            selected_row,
            direction,
            voices,
            performer_snapshots,
            scope,
            recorder_status,
            next_recording_armed,
            phrase_count,
            midi_clock,
            midi_channels,
            midi_transpose,
            ..
        } = view;

        canvas.set_draw_color(background());
        canvas.clear();

        if let Some(field) = setting_focus {
            return draw_setup(canvas, &view, field, self.brick);
        }
        draw_header(canvas, settings, None, sync_status, "PERFORMANCE")?;
        draw_beat_strip(canvas, timing, beat_flash)?;
        draw_text(
            canvas,
            370,
            52,
            &format!(
                "LIVE GRID {live_grid}   {}",
                if self.brick { "SEL+R2" } else { "F9" }
            ),
            1,
            accent(),
        )?;
        draw_text(canvas, 615, 52, groove, 1, dim_text())?;
        if sync_status.mode == SyncMode::MidiIn {
            draw_text(canvas, 710, 52, sync_status.midi_status, 1, accent())?;
        }
        draw_text(canvas, 815, 52, midi_clock, 1, dim_text())?;

        draw_panel(canvas, Rect::new(32, 82, 250, 220), panel())?;
        draw_text(canvas, 54, 100, "CURRENT", 2, dim_text())?;
        draw_text(
            canvas,
            54,
            140,
            &settings.tonal.chord_symbol(current_chord),
            5,
            text_color(),
        )?;
        draw_text(
            canvas,
            54,
            192,
            &format!(
                "{} {}",
                if self.brick { "R3" } else { "F6" },
                voicing.label()
            ),
            2,
            accent(),
        )?;
        if let Some(style) = pending_voicing {
            draw_text(
                canvas,
                54,
                207,
                &format!("NEXT BEAT {}", style.label()),
                1,
                accent(),
            )?;
        }

        let note_line = current_render
            .voicing
            .iter()
            .map(|note| settings.tonal.midi_note_label(note))
            .collect::<Vec<_>>()
            .join(" ");
        draw_text(canvas, 54, 225, &note_line, 2, text_color())?;

        let elapsed = if sync_status.running {
            format!("{:.1} BEATS", chord_elapsed_beats.max(0.0))
        } else {
            "TRANSPORT STOPPED".to_owned()
        };
        draw_text(canvas, 54, 263, &elapsed, 2, dim_text())?;

        draw_text(canvas, 315, 88, "NEXT", 2, dim_text())?;
        draw_recommendations(
            canvas,
            settings.tonal,
            recommendations,
            selected_class,
            selected_row,
        )?;

        draw_panel(canvas, Rect::new(780, 82, 212, 220), panel())?;
        draw_text(canvas, 800, 100, "DIRECTION", 2, dim_text())?;
        draw_text(canvas, 800, 123, direction.label(), 2, accent())?;
        let mut x = 800;
        for intent in [
            HarmonicDirection::Resolve,
            HarmonicDirection::Neutral,
            HarmonicDirection::Tension,
        ] {
            let color = if direction == intent {
                accent()
            } else {
                dim_text()
            };
            draw_text(canvas, x, 150, intent.label(), 1, color)?;
            x += 56;
        }
        draw_text(
            canvas,
            800,
            165,
            if self.brick { "L2 / R2" } else { "Q / E / W" },
            1,
            dim_text(),
        )?;
        draw_text(canvas, 800, 186, "BUTTON FOCUS", 1, dim_text())?;
        let focus_label = match scope {
            VoiceScope::All => "ALL",
            VoiceScope::One(index) => voices[index].role.label(),
        };
        draw_text(canvas, 800, 200, focus_label, 2, text_color())?;
        let macro_label = if has_macro_group {
            voices
                .iter()
                .zip(macro_targets)
                .filter(|(_, targeted)| *targeted)
                .map(|(voice, _)| voice.role.label())
                .collect::<Vec<_>>()
                .join("+")
        } else {
            focus_label.to_owned()
        };
        draw_text(canvas, 800, 228, "MACROS", 1, dim_text())?;
        draw_text(canvas, 800, 243, &macro_label, 1, accent())?;
        draw_text(canvas, 800, 274, "WHY", 1, dim_text())?;
        draw_text(
            canvas,
            835,
            274,
            recommendations.reason(selected_class, selected_row).label(),
            1,
            accent(),
        )?;

        draw_voice_rows(
            canvas,
            settings.tonal,
            voices,
            performer_snapshots,
            scope,
            (midi_channels, midi_transpose),
            (has_macro_group, macro_targets),
        )?;

        draw_panel(canvas, Rect::new(32, 660, 960, 67), panel())?;
        draw_recording_strip(
            canvas,
            recorder_status,
            next_recording_armed,
            phrase_count,
            self.brick,
        )?;
        let footer = if self.brick && select_held {
            let mute = if scope == VoiceScope::All {
                if performer_snapshots.iter().all(|state| state.muted) {
                    "UNMUTE ALL"
                } else {
                    "MUTE ALL"
                }
            } else if matches!(scope, VoiceScope::One(index) if performer_snapshots[index].muted) {
                "UNMUTE"
            } else {
                "MUTE"
            };
            let solo = if scope == VoiceScope::All
                || matches!(scope, VoiceScope::One(index) if performer_snapshots[index].soloed)
            {
                "CLEAR SOLO"
            } else {
                "SOLO"
            };
            format!(
                "L1 {mute}  R1 {solo}  L2 PATTERN HOLD  R2 LIVE GRID  R3 VARY  B ARM NEXT  X SETUP"
            )
        } else if self.brick {
            "A COMMIT   Y MACRO GROUP   L3 SUSTAIN   HOLD SELECT FOR SHORTCUTS".into()
        } else {
            "Z COMMIT   F4 MACRO GROUP   F5 SUSTAIN   M SETUP".into()
        };
        draw_text(canvas, 32, 739, &footer, 2, dim_text())?;
        Ok(())
    }

    pub fn draw_phrase_editor(
        &self,
        canvas: &mut sdl2::render::Canvas<sdl2::video::Window>,
        view: PhraseEditorView<'_>,
    ) -> Result<(), String> {
        let PhraseEditorView {
            timing,
            beat_flash,
            settings,
            sync_status,
            phrase,
            phrase_index,
            phrase_count,
            selected_event,
            pixels_per_beat,
            playing_phrase,
            queued_phrase,
            edit_grid,
            can_undo,
            select_held,
        } = view;

        canvas.set_draw_color(background());
        canvas.clear();

        draw_header(canvas, settings, None, sync_status, "PHRASE EDITOR")?;
        draw_beat_strip(canvas, timing, beat_flash)?;

        let Some(phrase) = phrase else {
            draw_text(canvas, 360, 310, "NO PHRASES YET", 4, dim_text())?;
            draw_text(
                canvas,
                270,
                370,
                if self.brick {
                    "MENU TO PERFORMANCE TO RECORD"
                } else {
                    "RECORD ON PERFORMANCE SCREEN"
                },
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

        draw_text(canvas, 42, 90, &previous, 3, dim_text())?;
        draw_text(canvas, 130, 90, &format!("[{}]", current), 3, text_color())?;
        draw_text(canvas, 260, 90, &next, 3, dim_text())?;

        if phrase.looping {
            draw_text(canvas, 850, 52, "LOOP ON", 1, accent())?;
        }
        let playback = playback_label(playing_phrase, queued_phrase);
        draw_text(canvas, 430, 90, &playback, 2, accent())?;

        draw_text(
            canvas,
            690,
            90,
            &format!("DUR GRID {}", edit_grid.label()),
            2,
            accent(),
        )?;
        draw_text(
            canvas,
            840,
            90,
            &format!("{:.1}B", phrase.duration_beats()),
            2,
            dim_text(),
        )?;

        draw_timeline(
            canvas,
            settings.tonal,
            phrase,
            selected_event,
            pixels_per_beat,
            self.width,
        )?;

        if let Some(event) = phrase.events.get(selected_event) {
            draw_panel(canvas, Rect::new(32, 610, 960, 112), panel())?;
            draw_text(canvas, 52, 626, "SELECTED", 2, dim_text())?;
            draw_text(
                canvas,
                52,
                660,
                &settings.tonal.chord_symbol(event.chord),
                5,
                text_color(),
            )?;
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
                535,
                635,
                if self.brick {
                    "A PLAY  B CHANGE  X DUP"
                } else {
                    "Z PLAY/QUEUE   X CHANGE   D DUP"
                },
                2,
                accent(),
            )?;
            draw_text(
                canvas,
                535,
                670,
                if self.brick {
                    "L/R2 SIZE  U/D GRID"
                } else {
                    "BACKSPACE DELETE  F/R SIZE  UP/DN GRID"
                },
                2,
                dim_text(),
            )?;
            draw_text(
                canvas,
                535,
                700,
                if self.brick {
                    "L/R1 PHRASE"
                } else {
                    "A/L PHRASE   -/= ZOOM   SPACE TRANSPORT"
                },
                2,
                dim_text(),
            )?;
        }

        draw_text(
            canvas,
            32,
            746,
            if self.brick && select_held {
                "Y DELETE   L/R1 ZOOM   X SETUP"
            } else if self.brick {
                "L3 LOOP   R3 CLEAR QUEUE   HOLD SELECT FOR SHORTCUTS"
            } else {
                "F5 LOOP   F6 CLEAR QUEUE"
            },
            1,
            dim_text(),
        )?;
        draw_text(
            canvas,
            530,
            746,
            if self.brick && select_held {
                "A UNDO"
            } else if self.brick {
                ""
            } else {
                "F12 UNDO"
            },
            1,
            if can_undo { accent() } else { dim_text() },
        )?;
        Ok(())
    }
}

fn draw_setup(
    canvas: &mut sdl2::render::Canvas<sdl2::video::Window>,
    view: &PerformanceView<'_>,
    field: SettingField,
    brick: bool,
) -> Result<(), String> {
    let sync = view.sync_status;
    let rig = field.is_rig();
    let live = sync.running || sync.starting;
    draw_header(
        canvas,
        view.settings,
        Some(field),
        sync,
        if live { "LIVE TEMPO" } else { "SETTINGS" },
    )?;
    if !live {
        draw_text(
            canvas,
            46,
            80,
            "MUSIC",
            2,
            if rig { dim_text() } else { accent() },
        )?;
        draw_text(
            canvas,
            158,
            80,
            "MIDI RIG",
            2,
            if rig { accent() } else { dim_text() },
        )?;
        draw_text(canvas, 320, 80, "X SWITCH TAB", 2, dim_text())?;
    }
    if rig {
        draw_rig_values(canvas, view, field, brick)?;
    } else {
        draw_panel(
            canvas,
            Rect::new(32, 114, 960, if live { 112 } else { 585 }),
            panel(),
        )?;
        let values = [
            view.settings.tonal.tonic.label().to_owned(),
            view.settings.tonal.mode.label().to_owned(),
            format!("{:.0} BPM", view.settings.tempo_bpm),
            view.settings.meter.label(),
            view.chord_gate.to_owned(),
            view.bass_arrival.to_owned(),
            view.groove.to_owned(),
            view.texture_role.to_owned(),
        ];
        for (row, ((setting, label), value)) in
            SettingField::MUSIC.into_iter().zip(values).enumerate()
        {
            if live && setting != SettingField::Tempo {
                continue;
            }
            let y = if live { 159 } else { 148 + row as i32 * 67 };
            draw_text(canvas, 58, y, label, 2, text_color())?;
            draw_setup_value(canvas, 378, y, 422, &value, setting == field)?;
        }
    }
    draw_text(
        canvas,
        32,
        734,
        if view.panic_confirm {
            if brick {
                "A SEND MIDI PANIC   B CANCEL"
            } else {
                "Z SEND MIDI PANIC   M CANCEL"
            }
        } else if brick {
            "D-PAD SELECT VALUE   L1 / R1 CHANGE   B BACK"
        } else {
            "ARROWS SELECT VALUE   - / = CHANGE   M BACK"
        },
        2,
        dim_text(),
    )
}

fn draw_rig_values(
    canvas: &mut sdl2::render::Canvas<sdl2::video::Window>,
    view: &PerformanceView<'_>,
    field: SettingField,
    brick: bool,
) -> Result<(), String> {
    let sync = view.sync_status;
    draw_panel(canvas, Rect::new(32, 112, 960, 235), panel())?;
    draw_text(canvas, 58, 132, "PERFORMER", 2, dim_text())?;
    draw_text(canvas, 325, 132, "CHANNEL", 2, dim_text())?;
    draw_text(canvas, 592, 132, "TRANSPOSE", 2, dim_text())?;
    for (index, voice) in view.voices.iter().enumerate() {
        let y = 174 + index as i32 * 42;
        draw_text(canvas, 58, y, voice.role.label(), 2, text_color())?;
        draw_setup_value(
            canvas,
            325,
            y,
            124,
            &view.midi_channels[index].to_string(),
            field == SettingField::MidiChannel(index),
        )?;
        draw_setup_value(
            canvas,
            592,
            y,
            124,
            &format!("{:+}", view.midi_transpose[index]),
            field == SettingField::MidiTranspose(index),
        )?;
    }
    let source = match sync.mode {
        SyncMode::Internal => "INTERNAL",
        SyncMode::MidiIn => "MIDI IN",
        SyncMode::Link => "ABLETON LINK",
    };
    let link_transport = if !sync.link_available {
        "LINK NOT BUILT"
    } else if sync.link_start_stop_sync {
        "ON"
    } else {
        "OFF"
    };
    let panic = if brick { "A REVIEW" } else { "Z REVIEW" };
    let values = [
        source,
        view.midi_clock,
        link_transport,
        view.midi_destination,
        panic,
    ];
    draw_panel(canvas, Rect::new(32, 360, 960, 348), panel())?;
    for (index, ((setting, label), value)) in SettingField::RIG.into_iter().zip(values).enumerate()
    {
        let y = 383 + index as i32 * 44;
        draw_text(canvas, 58, y, label, 2, text_color())?;
        let value: String = value.chars().take(46).collect();
        draw_setup_value(canvas, 378, y, 584, &value, setting == field)?;
    }
    let input = if sync.mode == SyncMode::MidiIn {
        format!("{} {}", sync.midi_status, view.midi_input.unwrap_or(""))
    } else {
        "INACTIVE".into()
    };
    let input: String = input.chars().take(46).collect();
    draw_text(canvas, 58, 615, "MIDI INPUT", 2, dim_text())?;
    draw_text(canvas, 378, 615, &input, 2, dim_text())?;
    draw_text(canvas, 58, 660, "LINK PEERS", 2, dim_text())?;
    draw_text(
        canvas,
        378,
        660,
        &sync.link_peers.to_string(),
        2,
        dim_text(),
    )
}

fn draw_setup_value(
    canvas: &mut sdl2::render::Canvas<sdl2::video::Window>,
    x: i32,
    y: i32,
    width: u32,
    value: &str,
    focused: bool,
) -> Result<(), String> {
    draw_panel(
        canvas,
        Rect::new(x - 8, y - 8, width, 31),
        if focused { accent() } else { dim_panel() },
    )?;
    draw_text(
        canvas,
        x,
        y,
        value,
        2,
        if focused { background() } else { text_color() },
    )
}

fn draw_beat_strip(
    canvas: &mut sdl2::render::Canvas<sdl2::video::Window>,
    timing: &str,
    flash: Option<bool>,
) -> Result<(), String> {
    if let Some(downbeat) = flash {
        draw_panel(
            canvas,
            Rect::new(84, 51, if downbeat { 16 } else { 8 }, 8),
            if downbeat { accent() } else { text_color() },
        )?;
    }
    draw_text(canvas, 106, 52, timing, 1, accent())
}

fn draw_header(
    canvas: &mut sdl2::render::Canvas<sdl2::video::Window>,
    settings: MusicalSettings,
    focus: Option<SettingField>,
    sync_status: SyncStatus,
    screen_label: &str,
) -> Result<(), String> {
    draw_text(canvas, 24, 20, "KHANA", 4, text_color())?;
    draw_text(canvas, 150, 23, screen_label, 2, dim_text())?;
    draw_text(
        canvas,
        24,
        52,
        concat!("V", env!("CARGO_PKG_VERSION")),
        1,
        dim_text(),
    )?;

    let tonic_color = field_color(focus, SettingField::Tonic);
    let mode_color = field_color(focus, SettingField::Mode);
    let tempo_color = field_color(focus, SettingField::Tempo);
    let meter_color = field_color(focus, SettingField::Meter);

    draw_text(
        canvas,
        360,
        23,
        settings.tonal.tonic.label(),
        2,
        tonic_color,
    )?;
    draw_text(canvas, 405, 23, settings.tonal.mode.label(), 2, mode_color)?;
    draw_text(
        canvas,
        600,
        23,
        &format!("{:.0} BPM", settings.tempo_bpm),
        2,
        tempo_color,
    )?;
    draw_text(canvas, 745, 23, &settings.meter.label(), 2, meter_color)?;

    let sync_label = match sync_status.mode {
        SyncMode::Internal => "INT".to_owned(),
        SyncMode::Link => format!("LINK {}", sync_status.link_peers),
        SyncMode::MidiIn => "MIDI IN".to_owned(),
    };
    draw_text(
        canvas,
        820,
        23,
        &sync_label,
        2,
        field_color(focus, SettingField::Sync),
    )?;
    draw_text(
        canvas,
        930,
        23,
        if sync_status.running {
            "PLAY"
        } else if sync_status.starting {
            "WAIT"
        } else {
            "STOP"
        },
        2,
        if sync_status.running || sync_status.starting {
            accent()
        } else {
            dim_text()
        },
    )?;

    Ok(())
}

fn field_color(focus: Option<SettingField>, field: SettingField) -> Color {
    if focus == Some(field) {
        accent()
    } else {
        dim_text()
    }
}

fn playback_label(playing_phrase: Option<usize>, queued_phrase: Option<usize>) -> String {
    let playing = playing_phrase
        .map(|index| format!("PLAY P{:02}", index + 1))
        .unwrap_or_else(|| "PLAY --".to_owned());
    let queued = queued_phrase
        .map(|index| format!("QUEUE P{:02}", index + 1))
        .unwrap_or_else(|| "QUEUE --".to_owned());

    format!("{playing}  {queued}")
}

fn draw_recommendations(
    canvas: &mut sdl2::render::Canvas<sdl2::video::Window>,
    context: crate::music::settings::TonalContext,
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
                &context.chord_symbol(chord),
                2,
                if selected { background() } else { text_color() },
            )?;
        }
    }

    Ok(())
}

fn draw_voice_rows(
    canvas: &mut sdl2::render::Canvas<sdl2::video::Window>,
    context: crate::music::settings::TonalContext,
    voices: &[Voice; 4],
    snapshots: [PerformerSnapshot; 4],
    scope: VoiceScope,
    (channels, transpose): ([u8; 4], [i8; 4]),
    (has_macro_group, macro_targets): (bool, [bool; 4]),
) -> Result<(), String> {
    draw_text(canvas, 36, 330, "PERFORMERS", 2, dim_text())?;
    draw_text(canvas, 260, 330, "DENS", 2, dim_text())?;
    draw_text(canvas, 430, 330, "DYN", 2, dim_text())?;
    draw_text(canvas, 600, 330, "SPRD", 2, dim_text())?;
    draw_text(canvas, 780, 330, "COLOR", 2, dim_text())?;

    let solo_active = snapshots.iter().any(|state| state.soloed);
    for (index, voice) in voices.iter().enumerate() {
        let state = snapshots[index];
        let quiet = state.muted || solo_active && !state.soloed;
        let y = 370 + index as i32 * 66;
        let scoped = scope == VoiceScope::All || scope == VoiceScope::One(index);

        draw_panel(
            canvas,
            Rect::new(32, y - 8, 960, 56),
            if scoped { scoped_panel() } else { panel() },
        )?;
        if scope == VoiceScope::One(index) {
            canvas.set_draw_color(accent());
            canvas.draw_rect(Rect::new(32, y - 8, 960, 56))?;
        }
        if has_macro_group && macro_targets[index] {
            draw_panel(canvas, Rect::new(39, y + 7, 7, 7), accent())?;
        }
        let active = active_note_summary(context, state.active_notes);
        draw_text(
            canvas,
            52,
            y + 4,
            &format!("{} {} {}", index + 1, voice.role.label(), active),
            2,
            if quiet { dim_text() } else { text_color() },
        )?;
        draw_text(
            canvas,
            52,
            y + 31,
            &format!(
                "CH {} TR {:+} {} {}",
                channels[index],
                transpose[index],
                state.reset_rule.label(),
                state.family.label(),
            ),
            1,
            dim_text(),
        )?;
        let mut status_x = 435;
        for (visible, label) in [
            (state.muted, "MUTE"),
            (state.soloed, "SOLO"),
            (state.sustain, "SUS"),
            (state.held, "HOLD"),
            (state.vary_queued, "VARY"),
        ] {
            if visible {
                draw_text(canvas, status_x, y + 40, label, 1, accent())?;
                status_x += 48;
            }
        }
        draw_value_bar(canvas, 250, y + 4, voice.parameters.density)?;
        draw_value_bar(canvas, 420, y + 4, voice.parameters.dynamics)?;
        draw_value_bar(canvas, 590, y + 4, voice.parameters.spread)?;
        draw_value_bar(canvas, 770, y + 4, voice.parameters.color)?;
    }

    Ok(())
}

fn active_note_summary(
    context: crate::music::settings::TonalContext,
    notes: [Option<u8>; MAX_VOICING_NOTES],
) -> String {
    let labels = notes
        .into_iter()
        .flatten()
        .map(|note| context.midi_note_label(note))
        .collect::<Vec<_>>();

    if labels.is_empty() {
        "--".to_owned()
    } else if labels.len() == 1 {
        labels[0].clone()
    } else {
        format!("{}N", labels.len())
    }
}

fn draw_recording_strip(
    canvas: &mut sdl2::render::Canvas<sdl2::video::Window>,
    status: RecorderStatus,
    next_armed: bool,
    phrase_count: usize,
    brick: bool,
) -> Result<(), String> {
    let capture = match status {
        RecorderStatus::Idle => "READY".to_owned(),
        RecorderStatus::Armed { .. } => "ARMED".to_owned(),
        RecorderStatus::Recording { phrase_id } => format!("RECORDING P{phrase_id:02}"),
        RecorderStatus::Stopping { phrase_id, .. } => format!("STOPPING P{phrase_id:02}"),
    };
    draw_text(canvas, 52, 686, "CAPTURE", 2, dim_text())?;
    draw_text(canvas, 180, 686, &capture, 2, accent())?;
    if next_armed {
        draw_text(canvas, 410, 686, "NEXT ARMED", 2, accent())?;
    }
    draw_text(
        canvas,
        635,
        686,
        &format!("{phrase_count} SAVED"),
        2,
        dim_text(),
    )?;
    draw_text(
        canvas,
        815,
        686,
        if brick { "SEL+A REC" } else { "P REC" },
        2,
        dim_text(),
    )
}

fn draw_timeline(
    canvas: &mut sdl2::render::Canvas<sdl2::video::Window>,
    context: crate::music::settings::TonalContext,
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
            &context.chord_symbol(event.chord),
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
