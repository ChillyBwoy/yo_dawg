use super::{EditTarget, Projection};
use crate::{
    app::{Action, App},
    model::Note,
};
use anyhow::{Context, Result, ensure};

pub fn pitch_name(key: u8) -> String {
    format!(
        "{}{}",
        [
            "C", "C#", "D", "D#", "E", "F", "F#", "G", "G#", "A", "A#", "B"
        ][key as usize % 12],
        key as i16 / 12 - 1
    )
}

fn parse_pitch(text: &str) -> Result<u8> {
    if let Ok(key) = text.parse::<u8>() {
        ensure!(key < 128, "MIDI key must be 0..127");
        return Ok(key);
    }
    let first = text
        .chars()
        .next()
        .context("Enter a pitch such as C4, F#3, or MIDI key 60")?;
    let semitone = match first.to_ascii_uppercase() {
        'C' => 0,
        'D' => 2,
        'E' => 4,
        'F' => 5,
        'G' => 7,
        'A' => 9,
        'B' => 11,
        _ => anyhow::bail!("Expected a pitch such as C4 or MIDI key 60"),
    };
    let rest = &text[first.len_utf8()..];
    let (accidental, octave) = if let Some(rest) = rest.strip_prefix('#') {
        (1, rest)
    } else if let Some(rest) = rest.strip_prefix('b') {
        (-1, rest)
    } else {
        (0, rest)
    };
    let octave: i32 = octave
        .parse()
        .context("Pitch needs an octave, for example C4")?;
    ensure!((-1..=9).contains(&octave), "Octave must be -1..9");
    let key = (octave + 1) * 12 + semitone + accidental;
    ensure!((0..128).contains(&key), "Pitch is outside MIDI range");
    Ok(key as u8)
}

impl App {
    pub(super) fn edit_tracker_note(&mut self, force_new: bool) {
        let (index, note) = self.note_for_tracker_edit(force_new);
        self.tracker.edit_target = Some(EditTarget {
            track: self.track,
            index,
        });
        self.prompt(
            "Note: pitch velocity length start (ticks)",
            format!(
                "{} {} {} {}",
                pitch_name(note.key),
                note.velocity,
                note.duration,
                note.start
            ),
            Action::EditNote,
        );
    }

    pub(in crate::app) fn apply_tracker_note(&mut self, text: &str) -> Result<()> {
        let target = self
            .tracker
            .edit_target
            .take()
            .context("No tracker note selected")?;
        let fields: Vec<_> = text.split_whitespace().collect();
        ensure!(
            fields.len() == 4,
            "Use: pitch velocity length start; e.g. C4 100 480 0"
        );
        let note = Note {
            key: parse_pitch(fields[0])?,
            velocity: fields[1].parse()?,
            duration: fields[2].parse()?,
            start: fields[3].parse()?,
        };
        let mut candidate = self.project.clone();
        let notes = &mut candidate
            .tracks
            .get_mut(target.track)
            .context("Track no longer exists")?
            .notes;
        let index = if let Some(index) = target.index {
            *notes.get_mut(index).context("Note no longer exists")? = note.clone();
            index
        } else {
            notes.push(note.clone());
            notes.len() - 1
        };
        candidate.validate()?;
        self.checkpoint();
        self.project = candidate;
        self.cursor = note.start;
        self.key = note.key;
        self.selected.clear();
        self.selected.insert(index);
        let projection = Projection::new(&self.project, self.grid);
        if let Some(column) = projection
            .columns
            .iter()
            .find(|c| c.track == target.track && c.notes.contains(&index))
        {
            self.tracker.lane = column.lane;
        }
        self.ensure_tracker_column_visible();
        self.sync();
        Ok(())
    }
}
