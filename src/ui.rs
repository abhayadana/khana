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
    pub options: &'a str,
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
    pub quantize_beats: f64,
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
            options,
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
            quantize_beats,
            phrase_count,
            midi_destination,
            midi_clock,
            midi_channels,
            midi_transpose,
            midi_input,
        } = view;

        canvas.set_draw_color(background());
        canvas.clear();

        if let Some(field) = setting_focus.filter(|field| field.is_rig()) {
            return draw_rig(canvas, &view, field, self.brick);
        }
        draw_header(canvas, settings, setting_focus, sync_status, "PERFORMANCE")?;
        draw_beat_strip(canvas, timing, beat_flash)?;
        let settings_hint = match setting_focus {
            Some(SettingField::Tonic) => Some("LEFT: MIDI RIG".to_owned()),
            Some(SettingField::ChordHold | SettingField::BassArrival | SettingField::Voicing) => {
                Some(options.to_owned())
            }

            Some(SettingField::Tempo) if sync_status.mode == SyncMode::MidiIn => {
                Some("TEMPO FROM MIDI INPUT".to_owned())
            }

            Some(SettingField::Tempo) if sync_status.running => {
                Some("LIVE TEMPO: UP/DOWN 1 BPM".to_owned())
            }
            _ => None,
        };
        if let Some(hint) = settings_hint {
            draw_text(canvas, 360, 52, &hint, 1, accent())?;
        } else if setting_focus.is_none()
            && let Some(input) = midi_input
        {
            let label: String = input.chars().take(42).collect();
            draw_text(
                canvas,
                360,
                52,
                &format!("{}  {label}", sync_status.midi_status),
                1,
                accent(),
            )?;
        }
        draw_text(
            canvas,
            815,
            52,
            midi_clock,
            1,
            field_color(setting_focus, SettingField::MidiClockOut),
        )?;

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
            &format!("VOICING {}", voicing.label()),
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
        draw_text(canvas, 800, 132, direction.label(), 3, accent())?;
        draw_text(
            canvas,
            800,
            158,
            if self.brick {
                "L2 RESOLVE  BOTH/Y NEUTRAL"
            } else {
                "Q RESOLVE  E NEUTRAL"
            },
            1,
            dim_text(),
        )?;
        draw_text(
            canvas,
            800,
            171,
            if self.brick {
                "R2 TENSION"
            } else {
                "W TENSION"
            },
            1,
            dim_text(),
        )?;
        draw_text(canvas, 800, 194, "SCOPE", 2, dim_text())?;
        draw_text(canvas, 800, 230, &scope.label(), 4, text_color())?;
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
        )?;

        draw_panel(canvas, Rect::new(32, 660, 960, 82), panel())?;
        draw_recording_strip(
            canvas,
            recorder_status,
            next_recording_armed,
            quantize_beats,
            phrase_count,
            midi_destination,
        )?;
        if setting_focus.is_none() {
            draw_text(
                canvas,
                52,
                729,
                if self.brick {
                    "L3 SUSTAIN  R3 VOICING  SEL+L/R2 MUTE/SOLO  SEL+L/R1 HOLD/GRID  SEL+R3 VARY"
                } else {
                    "F5 SUSTAIN  F6 VOICING  F7 VARY  F8 HOLD  F9 GRID  F10 MUTE  F11 SOLO"
                },
                1,
                dim_text(),
            )?;
        }
        if self.brick {
            let hints = if setting_focus.is_some() && sync_status.running {
                "U/D BPM  A/B/MENU CLOSE  START STOP"
            } else if setting_focus.is_some() {
                "L/R FIELD  U/D VALUE  A/B/MENU CLOSE  MENU HOLD EXIT"
            } else {
                "A COMMIT B REROLL X SURPRISE Y NEUTRAL SEL+A REC SEL+B ARM SEL+X SETUP MENU PAGE/HOLD EXIT"
            };
            draw_text(canvas, 32, 746, hints, 1, dim_text())?;
        }

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
                    "MENU: PERFORMANCE  SEL+A: RECORD"
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
            &format!("GRID {}", edit_grid.label()),
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
                    "SEL+Y DELETE  L/R2 SIZE  U/D GRID"
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
                    "L/R1 PHRASE  SEL+L/R1 ZOOM"
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
            if self.brick {
                "L3/SEL+X LOOP  R3/SEL+B CLEAR QUEUE"
            } else {
                "F5 LOOP  F6 CLEAR QUEUE"
            },
            1,
            dim_text(),
        )?;
        draw_text(
            canvas,
            530,
            746,
            if self.brick { "SEL+A UNDO" } else { "F12 UNDO" },
            1,
            if can_undo { accent() } else { dim_text() },
        )?;
        Ok(())
    }
}

fn draw_rig(
    canvas: &mut sdl2::render::Canvas<sdl2::video::Window>,
    view: &PerformanceView<'_>,
    field: SettingField,
    brick: bool,
) -> Result<(), String> {
    let sync = view.sync_status;
    draw_header(canvas, view.settings, Some(field), sync, "MIDI RIG")?;
    draw_panel(canvas, Rect::new(32, 82, 960, 278), panel())?;
    draw_text(canvas, 58, 102, "PERFORMER", 2, dim_text())?;
    draw_text(canvas, 370, 102, "MIDI CHANNEL", 2, dim_text())?;
    draw_text(canvas, 670, 102, "TRANSPOSE (SEMITONES)", 2, dim_text())?;

    for (index, voice) in view.voices.iter().enumerate() {
        let y = 145 + index as i32 * 48;
        draw_text(canvas, 58, y, voice.role.label(), 2, text_color())?;
        draw_text(
            canvas,
            370,
            y,
            &view.midi_channels[index].to_string(),
            2,
            rig_field_color(field, SettingField::MidiChannel(index)),
        )?;
        draw_text(
            canvas,
            670,
            y,
            &format!("{:+}", view.midi_transpose[index]),
            2,
            rig_field_color(field, SettingField::MidiTranspose(index)),
        )?;
    }

    let clock_source = match sync.mode {
        SyncMode::Internal => "INTERNAL",
        SyncMode::MidiIn => "MIDI IN",
        SyncMode::Link => "ABLETON LINK",
    };
    let link = if sync.link_available {
        format!(
            "{} PEERS   START/STOP {}",
            sync.link_peers,
            if sync.link_start_stop_sync {
                "ON"
            } else {
                "OFF"
            }
        )
    } else {
        "LINK NOT BUILT".into()
    };
    let input = if sync.mode == SyncMode::MidiIn {
        match view.midi_input {
            Some(input) => format!("{}   {input}", sync.midi_status),
            None => sync.midi_status.to_owned(),
        }
    } else {
        "INACTIVE".into()
    };
    let focus = Some(field);
    draw_panel(canvas, Rect::new(32, 383, 960, 327), panel())?;
    for (index, (label, value, setting)) in [
        (
            "CLOCK SOURCE",
            clock_source.to_owned(),
            Some(SettingField::Sync),
        ),
        ("MIDI CLOCK IN", input, None),
        (
            "MIDI CLOCK OUT",
            view.midi_clock.to_owned(),
            Some(SettingField::MidiClockOut),
        ),
        ("ABLETON LINK", link, Some(SettingField::StartStopSync)),
        (
            "MIDI OUTPUT",
            view.midi_destination.to_owned(),
            Some(SettingField::MidiOutput),
        ),
        (
            "MIDI PANIC",
            if brick {
                "A TO SEND".into()
            } else {
                "Z TO SEND".into()
            },
            Some(SettingField::MidiPanic),
        ),
    ]
    .into_iter()
    .enumerate()
    {
        let y = 402 + index as i32 * 47;
        let color = if setting == focus {
            accent()
        } else {
            text_color()
        };
        draw_text(canvas, 58, y, label, 2, color)?;
        let value: String = value.chars().take(48).collect();
        draw_text(canvas, 370, y, &value, 2, color)?;
    }

    draw_text(
        canvas,
        32,
        734,
        if brick {
            "L/R FIELD  U/D VALUE  A/B/MENU CLOSE"
        } else {
            "LEFT/RIGHT FIELD  UP/DOWN VALUE  Z/M CLOSE"
        },
        1,
        dim_text(),
    )
}

fn rig_field_color(focus: SettingField, field: SettingField) -> Color {
    if focus == field {
        accent()
    } else {
        text_color()
    }
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
    quantize_beats: f64,
    phrase_count: usize,
    midi_destination: &str,
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
        &format!("Q {:.1} BEATS", quantize_beats),
        2,
        dim_text(),
    )?;
    draw_text(
        canvas,
        820,
        678,
        &format!("SAVED {}", phrase_count),
        2,
        dim_text(),
    )?;

    let capture_label = if next_armed {
        "NEXT PHRASE ARMED"
    } else {
        "CAPTURE READY"
    };
    let midi_label = midi_destination.chars().take(32).collect::<String>();
    draw_text(
        canvas,
        180,
        712,
        &format!("{capture_label}   MIDI {midi_label}"),
        1,
        dim_text(),
    )?;

    Ok(())
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
