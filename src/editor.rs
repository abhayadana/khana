//! Phrase Editor chord selection and structural edits.

use crate::music::chord::{Chord, ChordExtension, ChordQuality, PitchClass};
use crate::music::recommendation::{HarmonicDirection, RecommendationClass, RecommendationSet};
use crate::phrase::{ChainSection, ChordEvent, Phrase, Transition};
use crate::{App, InputAction};

const QUALITIES: [ChordQuality; 7] = [
    ChordQuality::Major,
    ChordQuality::Minor,
    ChordQuality::Dominant,
    ChordQuality::Diminished,
    ChordQuality::HalfDiminished,
    ChordQuality::MinorMajor,
    ChordQuality::AugmentedMajor,
];
const EXTENSIONS: [ChordExtension; 3] = [
    ChordExtension::Triad,
    ChordExtension::Seventh,
    ChordExtension::Ninth,
];

#[derive(Clone, Copy)]
pub enum ChordEdit {
    Replace,
    InsertBefore,
    InsertAfter,
}

impl ChordEdit {
    pub const fn title(self) -> &'static str {
        match self {
            Self::Replace => "CHANGE CHORD",
            Self::InsertBefore => "INSERT BEFORE",
            Self::InsertAfter => "INSERT AFTER",
        }
    }
}

#[derive(Clone)]
pub struct ChordPicker {
    pub operation: ChordEdit,
    pub options: RecommendationSet,
    pub class: usize,
    pub row: usize,
    pub manual: bool,
    pub field: usize,
    pub chord: Chord,
    pub previous: Option<Chord>,
    pub following: Option<Chord>,
    pub duration: f64,
}

impl ChordPicker {
    fn new(app: &App, operation: ChordEdit) -> Option<Self> {
        let phrase = app.selected_phrase()?;
        let index = app.editor_event_index;
        let event = phrase.events.get(index)?;
        let (before, after, duration) = match operation {
            ChordEdit::Replace => (index, index + 1, event.duration_beats),
            ChordEdit::InsertBefore => (index, index, app.duration_grid_beats()),
            ChordEdit::InsertAfter => (index + 1, index + 1, app.duration_grid_beats()),
        };
        let previous = before.checked_sub(1).map(|i| phrase.events[i].chord);
        let following = phrase.events.get(after).map(|entry| entry.chord);
        let mut history: Vec<_> = phrase.events[..before]
            .iter()
            .map(|entry| entry.chord)
            .collect();
        if history.is_empty() {
            history.push(app.settings.tonal.tonic_chord());
        }
        let options = app.recommendation_engine.recommend(
            &history,
            app.settings.tonal,
            HarmonicDirection::Neutral,
            false,
        );
        Some(Self {
            operation,
            options,
            class: 0,
            row: 0,
            manual: false,
            field: 0,
            chord: event.chord,
            previous,
            following,
            duration,
        })
    }

    fn chosen(&self) -> Chord {
        if self.manual {
            self.chord
        } else {
            self.options
                .get(RecommendationClass::ALL[self.class], self.row)
        }
    }

    fn step(&mut self, direction: i32) {
        if !self.manual {
            self.row = (self.row as i32 + direction).clamp(0, 2) as usize;
            return;
        }
        match self.field {
            0 => {
                let pitch = (i32::from(self.chord.root.value()) - direction).rem_euclid(12);
                self.chord.root = PitchClass::from_value(pitch as u8);
            }
            1 => self.chord.quality = cycle(QUALITIES, self.chord.quality, -direction),
            _ => self.chord.extension = cycle(EXTENSIONS, self.chord.extension, -direction),
        }
    }

    pub fn manual_values(&self) -> [&'static str; 3] {
        let quality = match self.chord.quality {
            ChordQuality::Major => "MAJOR",
            ChordQuality::Minor => "MINOR",
            ChordQuality::Dominant => "DOMINANT",
            ChordQuality::Diminished => "DIMINISHED",
            ChordQuality::HalfDiminished => "HALF-DIM",
            ChordQuality::MinorMajor => "MINOR-MAJOR",
            ChordQuality::AugmentedMajor => "AUG-MAJOR",
        };
        let extension = match self.chord.extension {
            ChordExtension::Triad => "TRIAD",
            ChordExtension::Seventh => "7TH",
            ChordExtension::Ninth => "9TH",
        };
        [self.chord.root.label(), quality, extension]
    }
}

fn cycle<T: Copy + PartialEq, const N: usize>(items: [T; N], value: T, direction: i32) -> T {
    let index = items.iter().position(|&item| item == value).unwrap_or(0);
    items[(index as i32 + direction).rem_euclid(N as i32) as usize]
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum EditItem {
    InsertBefore,
    InsertAfter,
    Duplicate,
    Copy,
    Cut,
    Paste,
    Delete,
    Fit,
    Chain,
}

impl EditItem {
    pub const ALL: [Self; 9] = [
        Self::InsertBefore,
        Self::InsertAfter,
        Self::Duplicate,
        Self::Copy,
        Self::Cut,
        Self::Paste,
        Self::Delete,
        Self::Fit,
        Self::Chain,
    ];

    pub const fn label(self, fit: bool) -> &'static str {
        match self {
            Self::InsertBefore => "INSERT BEFORE",
            Self::InsertAfter => "INSERT AFTER",
            Self::Duplicate => "DUPLICATE",
            Self::Copy => "COPY",
            Self::Cut => "CUT",
            Self::Paste => "PASTE AFTER",
            Self::Delete => "DELETE",
            Self::Fit if fit => "RESTORE ZOOM",
            Self::Fit => "FIT PHRASE",
            Self::Chain => "QUEUE SECTION",
        }
    }

    pub fn enabled(self, phrase: &Phrase, has_clipboard: bool) -> bool {
        match self {
            Self::Cut | Self::Delete => phrase.events.len() > 1,
            Self::Paste => has_clipboard,
            _ => true,
        }
    }
}

#[derive(Clone)]
pub enum EditorModal {
    Edit(usize),
    Chord(ChordPicker),
    Chain {
        passes: u8,
        transition: Transition,
        choose_transition: bool,
    },
}

impl App {
    pub fn duration_grid_beats(&self) -> f64 {
        self.editor_grid.beats(
            self.settings.meter.numerator,
            self.settings.meter.denominator,
        )
    }

    pub fn open_chord_picker(&mut self, operation: ChordEdit) {
        self.editor_modal = ChordPicker::new(self, operation).map(EditorModal::Chord);
    }

    fn apply_chord_edit(&mut self, picker: &ChordPicker) {
        let chord = picker.chosen();
        match picker.operation {
            ChordEdit::Replace => {
                self.edit_phrase(|phrase, index| phrase.replace_chord(index, chord));
            }
            ChordEdit::InsertBefore | ChordEdit::InsertAfter => {
                self.insert_editor_event(
                    ChordEvent {
                        chord,
                        start_beat: 0.0,
                        duration_beats: picker.duration,
                    },
                    matches!(picker.operation, ChordEdit::InsertAfter),
                );
            }
        }
    }

    fn insert_editor_event(&mut self, event: ChordEvent, after: bool) {
        if self.edit_phrase(|phrase, index| phrase.insert_event(index + usize::from(after), event))
        {
            self.editor_event_index += usize::from(after);
            self.clamp_editor_selection();
        }
    }

    fn apply_edit_item(&mut self, item: EditItem) {
        let Some(phrase) = self.selected_phrase() else {
            return;
        };
        if !item.enabled(phrase, self.editor_clipboard.is_some()) {
            return;
        }
        match item {
            EditItem::InsertBefore => self.open_chord_picker(ChordEdit::InsertBefore),
            EditItem::InsertAfter => self.open_chord_picker(ChordEdit::InsertAfter),
            EditItem::Duplicate => self.duplicate_editor_event(),
            EditItem::Copy | EditItem::Cut => {
                self.editor_clipboard = phrase.events.get(self.editor_event_index).copied();
                if matches!(item, EditItem::Cut) {
                    self.delete_editor_event();
                }
            }
            EditItem::Paste => {
                if let Some(event) = self.editor_clipboard {
                    self.insert_editor_event(event, true);
                }
            }
            EditItem::Delete => self.delete_editor_event(),
            EditItem::Fit => self.editor_fit = !self.editor_fit,
            EditItem::Chain => {
                self.editor_modal = Some(EditorModal::Chain {
                    passes: 1,
                    transition: Transition::PhraseEnd,
                    choose_transition: false,
                });
            }
        }
    }

    pub fn duplicate_editor_event(&mut self) {
        if self.edit_phrase(Phrase::duplicate_event) {
            self.editor_event_index += 1;
            self.clamp_editor_selection();
        }
    }

    pub fn delete_editor_event(&mut self) {
        if self.edit_phrase(Phrase::delete_event) {
            self.clamp_editor_selection();
        }
    }

    /// Handles editor-only inputs. A completed chain setup is returned to the
    /// caller so it can apply transport rules at the same entry point as Play.
    pub fn handle_editor_modal(&mut self, action: InputAction) -> Option<ChainSection> {
        use InputAction::*;
        let mut modal = self.editor_modal.take()?;
        if matches!(action, Secondary | MenuTap | ToggleScreen | ToggleSettings) {
            return None;
        }
        match &mut modal {
            EditorModal::Edit(row) => match action {
                NavigateUp => *row = row.saturating_sub(1),
                NavigateDown => *row = (*row + 1).min(EditItem::ALL.len() - 1),
                Primary => {
                    let item = EditItem::ALL[*row];
                    if self
                        .selected_phrase()
                        .is_some_and(|phrase| item.enabled(phrase, self.editor_clipboard.is_some()))
                    {
                        self.apply_edit_item(item);
                        return None;
                    }
                }
                _ => {}
            },
            EditorModal::Chord(picker) => match action {
                NavigateLeft | NavigateRight => {
                    let target = if picker.manual {
                        &mut picker.field
                    } else {
                        &mut picker.class
                    };
                    if action == NavigateLeft {
                        *target = target.saturating_sub(1);
                    } else {
                        *target = (*target + 1).min(2);
                    }
                }
                NavigateUp => picker.step(-1),
                NavigateDown => picker.step(1),
                Tertiary => picker.manual = !picker.manual,
                Primary => {
                    self.apply_chord_edit(picker);
                    return None;
                }
                _ => {}
            },
            EditorModal::Chain {
                passes,
                transition,
                choose_transition,
            } => match action {
                NavigateLeft => *choose_transition = false,
                NavigateRight => *choose_transition = true,
                NavigateUp | NavigateDown if *choose_transition => {
                    *transition = match transition {
                        Transition::PhraseEnd => Transition::Bar,
                        Transition::Bar => Transition::PhraseEnd,
                    };
                }
                NavigateUp => *passes = (*passes + 1).min(16),
                NavigateDown => *passes = passes.saturating_sub(1).max(1),
                Primary => {
                    return Some(ChainSection {
                        phrase_index: self.editor_phrase_index,
                        passes: *passes,
                        transition: *transition,
                    });
                }
                _ => {}
            },
        }
        self.editor_modal = Some(modal);
        None
    }
}
