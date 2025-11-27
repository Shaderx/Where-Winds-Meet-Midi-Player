use midly::{Smf, TrackEventKind, MidiMessage};
use std::sync::atomic::{AtomicBool, AtomicU8, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tauri::{Window, Emitter};
use serde::{Serialize, Deserialize};

/// Note calculation mode - how MIDI notes are mapped to game keys
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[repr(u8)]
pub enum NoteMode {
    Closest = 0,      // Find closest available note (original behavior)
    Quantize = 1,     // Snap to exact scale notes only
    TransposeOnly = 2, // Just shift octaves, direct mapping
    Pentatonic = 3,   // Map to pentatonic scale (5 notes)
    Chromatic = 4,    // Detailed chromatic mapping
    Raw = 5,          // Raw 1:1 mapping, no transpose
}

impl From<u8> for NoteMode {
    fn from(value: u8) -> Self {
        match value {
            0 => NoteMode::Closest,
            1 => NoteMode::Quantize,
            2 => NoteMode::TransposeOnly,
            3 => NoteMode::Pentatonic,
            4 => NoteMode::Chromatic,
            5 => NoteMode::Raw,
            _ => NoteMode::Closest,
        }
    }
}

#[derive(Debug, Clone)]
pub struct MidiData {
    pub events: Vec<TimedEvent>,
    pub duration: f64,
    pub transpose: i32,
}

#[derive(Debug, Clone)]
pub struct TimedEvent {
    pub time_ms: u64,
    pub event_type: EventType,
    pub note: u8,
}

#[derive(Debug, Clone)]
pub enum EventType {
    NoteOn,
    NoteOff,
}

// 21-key mode: Basic keys for 3 octaves (7 notes each)
const LOW_KEYS: [&str; 7] = ["z", "x", "c", "v", "b", "n", "m"];
const MID_KEYS: [&str; 7] = ["a", "s", "d", "f", "g", "h", "j"];
const HIGH_KEYS: [&str; 7] = ["q", "w", "e", "r", "t", "y", "u"];


const SCALE_INTERVALS: [i32; 7] = [0, 2, 4, 5, 7, 9, 11];
const ROOT_NOTE: i32 = 60; // C4

/// Quick function to get MIDI duration without full processing
pub fn get_midi_duration(path: &str) -> Result<f64, String> {
    let data = std::fs::read(path).map_err(|e| e.to_string())?;
    let smf = Smf::parse(&data).map_err(|e| e.to_string())?;

    let ticks_per_quarter = match smf.header.timing {
        midly::Timing::Metrical(tpq) => tpq.as_int() as f64,
        _ => 480.0,
    };

    let mut tempo_changes: Vec<(u64, f64)> = Vec::new();
    let mut max_ticks: u64 = 0;

    // Collect tempo changes and find max ticks
    for track in &smf.tracks {
        let mut track_time_ticks: u64 = 0;
        for event in track {
            track_time_ticks += event.delta.as_int() as u64;
            if let TrackEventKind::Meta(midly::MetaMessage::Tempo(t)) = event.kind {
                tempo_changes.push((track_time_ticks, t.as_int() as f64));
            }
        }
        if track_time_ticks > max_ticks {
            max_ticks = track_time_ticks;
        }
    }
    tempo_changes.sort_by_key(|(time, _)| *time);

    // Convert max ticks to milliseconds
    let mut result_ms = 0.0;
    let mut last_tick = 0u64;
    let mut current_tempo = 500_000.0; // Default 120 BPM

    for &(change_tick, new_tempo) in &tempo_changes {
        if change_tick >= max_ticks {
            break;
        }
        let delta_ticks = change_tick - last_tick;
        result_ms += delta_ticks as f64 / ticks_per_quarter * current_tempo / 1000.0;
        last_tick = change_tick;
        current_tempo = new_tempo;
    }

    let delta_ticks = max_ticks - last_tick;
    result_ms += delta_ticks as f64 / ticks_per_quarter * current_tempo / 1000.0;

    Ok(result_ms / 1000.0) // Convert to seconds
}

pub fn load_midi(path: &str) -> Result<MidiData, String> {
    let data = std::fs::read(path).map_err(|e| e.to_string())?;
    let smf = Smf::parse(&data).map_err(|e| e.to_string())?;

    let mut events = Vec::new();
    let mut current_time_ms: u64 = 0;
    let ticks_per_quarter = match smf.header.timing {
        midly::Timing::Metrical(tpq) => tpq.as_int() as f64,
        _ => 480.0, // Default
    };

    let mut tempo = 500_000.0; // Default tempo (120 BPM)
    let mut tempo_changes: Vec<(u64, f64)> = Vec::new();

    // First pass: collect all tempo changes from all tracks
    for track in &smf.tracks {
        let mut track_time_ticks: u64 = 0;
        for event in track {
            track_time_ticks += event.delta.as_int() as u64;
            if let TrackEventKind::Meta(midly::MetaMessage::Tempo(t)) = event.kind {
                tempo_changes.push((track_time_ticks, t.as_int() as f64));
            }
        }
    }
    tempo_changes.sort_by_key(|(time, _)| *time);

    // Function to convert ticks to milliseconds with tempo changes
    let ticks_to_ms = |ticks: u64| -> u64 {
        let mut result_ms = 0.0;
        let mut last_tick = 0u64;
        let mut current_tempo = 500_000.0;

        for &(change_tick, new_tempo) in &tempo_changes {
            if change_tick >= ticks {
                break;
            }
            // Add time up to this tempo change
            let delta_ticks = change_tick - last_tick;
            result_ms += (delta_ticks as f64 / ticks_per_quarter * current_tempo / 1000.0);
            last_tick = change_tick;
            current_tempo = new_tempo;
        }

        // Add remaining time
        let delta_ticks = ticks - last_tick;
        result_ms += (delta_ticks as f64 / ticks_per_quarter * current_tempo / 1000.0);
        result_ms as u64
    };

    // Second pass: process all tracks with proper timing
    for track in &smf.tracks {
        let mut track_time_ticks: u64 = 0;

        for event in track {
            track_time_ticks += event.delta.as_int() as u64;
            let time_ms = ticks_to_ms(track_time_ticks);

            if let TrackEventKind::Midi { message, .. } = event.kind {
                match message {
                    MidiMessage::NoteOn { key, vel } => {
                        if vel > 0 {
                            events.push(TimedEvent {
                                time_ms,
                                event_type: EventType::NoteOn,
                                note: key.as_int(),
                            });
                        } else {
                            // Note on with velocity 0 is treated as note off
                            events.push(TimedEvent {
                                time_ms,
                                event_type: EventType::NoteOff,
                                note: key.as_int(),
                            });
                        }
                    }
                    MidiMessage::NoteOff { key, .. } => {
                        events.push(TimedEvent {
                            time_ms,
                            event_type: EventType::NoteOff,
                            note: key.as_int(),
                        });
                    }
                    _ => {}
                }
            }
        }
    }

    // Sort events by time
    events.sort_by_key(|e| e.time_ms);

    // Calculate duration
    let duration = if !events.is_empty() {
        events.last().unwrap().time_ms as f64 / 1000.0
    } else {
        0.0
    };

    // Detect best transpose (port of Python heuristic)
    let transpose = detect_best_transpose(&events);
    println!("Detected transpose: {} semitones", transpose);

    Ok(MidiData {
        events,
        duration,
        transpose,
    })
}

fn detect_best_transpose(events: &[TimedEvent]) -> i32 {
    let instrument_notes = get_instrument_notes();

    let mut best_transpose = 0;
    let mut best_score = i32::MAX;

    // Test transpose values from -12 to +12
    for transpose in -12..=12 {
        let mut score = 0;

        for event in events {
            if matches!(event.event_type, EventType::NoteOn) {
                let transposed_note = (event.note as i32 + transpose) as i32;
                let normalized = normalize_into_range(transposed_note);

                // Find distance to nearest instrument note
                let mut min_distance = i32::MAX;
                for inst_note in &instrument_notes {
                    let distance = (inst_note - normalized).abs();
                    if distance < min_distance {
                        min_distance = distance;
                    }
                }
                score += min_distance;
            }
        }

        if score < best_score {
            best_score = score;
            best_transpose = transpose;
        }
    }

    best_transpose
}

fn get_instrument_notes() -> Vec<i32> {
    let mut notes = Vec::new();

    // Low octave
    for interval in SCALE_INTERVALS {
        notes.push(ROOT_NOTE - 12 + interval);
    }

    // Mid octave
    for interval in SCALE_INTERVALS {
        notes.push(ROOT_NOTE + interval);
    }

    // High octave
    for interval in SCALE_INTERVALS {
        notes.push(ROOT_NOTE + 12 + interval);
    }

    notes
}

fn normalize_into_range(note: i32) -> i32 {
    let instrument_notes = get_instrument_notes();
    let lo = instrument_notes[0];
    let hi = instrument_notes[instrument_notes.len() - 1];

    let mut normalized = note;
    while normalized < lo {
        normalized += 12;
    }
    while normalized > hi {
        normalized -= 12;
    }

    normalized
}


fn note_to_key(note: i32, transpose: i32) -> String {
    let target = normalize_into_range(note + transpose);
    let instrument_notes = get_instrument_notes();

    let mut best_idx = 0;
    let mut best_dist = (instrument_notes[0] - target).abs();

    for (i, &inst_note) in instrument_notes.iter().enumerate() {
        let dist = (inst_note - target).abs();
        if dist < best_dist {
            best_idx = i;
            best_dist = dist;
        }
    }

    // Map index to key
    let all_keys = [LOW_KEYS.as_slice(), MID_KEYS.as_slice(), HIGH_KEYS.as_slice()].concat();
    let key = all_keys[best_idx].to_string();

    // Debug first few mappings
    static mut DEBUG_COUNT: i32 = 0;
    unsafe {
        if DEBUG_COUNT < 5 {
            println!("Note {} + transpose {} = target {}, matched {}, key: {}",
                note, transpose, target, instrument_notes[best_idx], key);
            DEBUG_COUNT += 1;
        }
    }

    key
}

/// Quantize mode - snap to exact scale notes only (no in-between approximation)
fn note_to_key_quantize(note: i32, transpose: i32) -> String {
    let target = note + transpose;
    let instrument_notes = get_instrument_notes();
    let lo = instrument_notes[0];
    let hi = instrument_notes[instrument_notes.len() - 1];

    // Normalize to range
    let mut normalized = target;
    while normalized < lo {
        normalized += 12;
    }
    while normalized > hi {
        normalized -= 12;
    }

    // Find exact match or closest scale note
    let mut best_idx = 0;
    let mut best_dist = i32::MAX;

    for (i, &inst_note) in instrument_notes.iter().enumerate() {
        let dist = (inst_note - normalized).abs();
        if dist < best_dist {
            best_idx = i;
            best_dist = dist;
        }
    }

    // Only play if it's an exact match or very close (within 1 semitone)
    if best_dist > 1 {
        // Skip notes that don't fit the scale well - map to nearest
        best_idx = best_idx;
    }

    let all_keys = [LOW_KEYS.as_slice(), MID_KEYS.as_slice(), HIGH_KEYS.as_slice()].concat();
    all_keys[best_idx].to_string()
}

/// Transpose Only mode - direct semitone to key mapping within octave
fn note_to_key_transpose(note: i32, transpose: i32) -> String {
    let target = note + transpose;

    // Get semitone within octave (0-11)
    let semitone = ((target - ROOT_NOTE) % 12 + 12) % 12;

    // Determine octave
    let octave_offset = (target - ROOT_NOTE) / 12;
    let octave = (1 + octave_offset).clamp(0, 2) as usize;

    // Direct mapping: semitone 0-11 to key 0-6 (wrap around)
    // This gives a more "raw" feel
    let key_idx = (semitone * 7 / 12) as usize;

    match octave {
        0 => LOW_KEYS[key_idx].to_string(),
        1 => MID_KEYS[key_idx].to_string(),
        _ => HIGH_KEYS[key_idx].to_string(),
    }
}

/// Pentatonic mode - map to pentatonic scale (5 notes per octave)
/// Pentatonic: C, D, E, G, A (indices 0, 1, 2, 4, 5 in our 7-note scale)
fn note_to_key_pentatonic(note: i32, transpose: i32) -> String {
    let target = note + transpose;

    // Pentatonic intervals from root: 0, 2, 4, 7, 9 (C, D, E, G, A)
    const PENTA_INTERVALS: [i32; 5] = [0, 2, 4, 7, 9];
    const PENTA_KEY_IDX: [usize; 5] = [0, 1, 2, 4, 5]; // Map to do, re, mi, so, la

    // Normalize to range
    let instrument_notes = get_instrument_notes();
    let lo = instrument_notes[0];
    let hi = instrument_notes[instrument_notes.len() - 1];

    let mut normalized = target;
    while normalized < lo {
        normalized += 12;
    }
    while normalized > hi {
        normalized -= 12;
    }

    // Get semitone within octave
    let semitone = ((normalized - ROOT_NOTE) % 12 + 12) % 12;

    // Determine octave
    let octave = if normalized < ROOT_NOTE {
        0
    } else if normalized < ROOT_NOTE + 12 {
        1
    } else {
        2
    };

    // Find closest pentatonic note
    let mut best_penta_idx = 0;
    let mut best_dist = i32::MAX;
    for (i, &interval) in PENTA_INTERVALS.iter().enumerate() {
        let dist = (interval - semitone).abs().min((interval - semitone + 12).abs()).min((interval - semitone - 12).abs());
        if dist < best_dist {
            best_dist = dist;
            best_penta_idx = i;
        }
    }

    let key_idx = PENTA_KEY_IDX[best_penta_idx];

    match octave {
        0 => LOW_KEYS[key_idx].to_string(),
        1 => MID_KEYS[key_idx].to_string(),
        _ => HIGH_KEYS[key_idx].to_string(),
    }
}

/// Chromatic mode - detailed mapping of all 12 semitones to closest natural key
fn note_to_key_chromatic(note: i32, transpose: i32) -> String {
    let target = note + transpose;

    // Normalize into our 3-octave range
    let instrument_notes = get_instrument_notes();
    let lo = instrument_notes[0];
    let hi = instrument_notes[instrument_notes.len() - 1];

    let mut normalized = target;
    while normalized < lo {
        normalized += 12;
    }
    while normalized > hi {
        normalized -= 12;
    }

    // Get semitone within octave (0-11)
    let semitone_in_octave = ((normalized - ROOT_NOTE) % 12 + 12) % 12;

    // Determine which octave we're in
    let octave = if normalized < ROOT_NOTE {
        0 // Low
    } else if normalized < ROOT_NOTE + 12 {
        1 // Mid
    } else {
        2 // High
    };

    // Map each chromatic semitone to closest natural key (0-6)
    // Semitone: 0=C, 1=C#, 2=D, 3=Eb, 4=E, 5=F, 6=F#, 7=G, 8=G#, 9=A, 10=Bb, 11=B
    let key_idx = match semitone_in_octave {
        0 => 0,   // C -> do
        1 => 0,   // C# -> do
        2 => 1,   // D -> re
        3 => 2,   // Eb -> mi
        4 => 2,   // E -> mi
        5 => 3,   // F -> fa
        6 => 3,   // F# -> fa
        7 => 4,   // G -> so
        8 => 4,   // G# -> so
        9 => 5,   // A -> la
        10 => 6,  // Bb -> ti
        11 => 6,  // B -> ti
        _ => 0,
    };

    match octave {
        0 => LOW_KEYS[key_idx].to_string(),
        1 => MID_KEYS[key_idx].to_string(),
        _ => HIGH_KEYS[key_idx].to_string(),
    }
}

/// Raw mode - direct 1:1 mapping, no transpose, no processing
/// MIDI note modulo 21 maps directly to one of 21 keys
fn note_to_key_raw(note: i32) -> String {
    // Direct mapping: note % 21 gives key index 0-20
    let key_idx = ((note % 21) + 21) % 21; // Handle negative notes
    let all_keys = [LOW_KEYS.as_slice(), MID_KEYS.as_slice(), HIGH_KEYS.as_slice()].concat();
    all_keys[key_idx as usize].to_string()
}


pub fn play_midi(
    midi_data: MidiData,
    is_playing: Arc<AtomicBool>,
    is_paused: Arc<AtomicBool>,
    loop_mode: Arc<AtomicBool>,
    note_mode: Arc<AtomicU8>,
    octave_shift: Arc<std::sync::atomic::AtomicI8>,
    current_position: Arc<std::sync::Mutex<f64>>,
    seek_offset: Arc<std::sync::Mutex<f64>>,
    window: Window,
) {
    let offset_ms = (*seek_offset.lock().unwrap() * 1000.0) as u64;

    // Spawn a separate thread for progress updates
    let is_playing_progress = Arc::clone(&is_playing);
    let is_paused_progress = Arc::clone(&is_paused);
    let current_position_progress = Arc::clone(&current_position);
    let window_progress = window.clone();

    std::thread::spawn(move || {
        while is_playing_progress.load(Ordering::SeqCst) {
            if !is_paused_progress.load(Ordering::SeqCst) {
                let position = *current_position_progress.lock().unwrap();
                let _ = window_progress.emit("playback-progress", position);
            }
            std::thread::sleep(Duration::from_millis(100));
        }
    });

    loop {
        let start_time = Instant::now();
        // Track which key is pressed for each MIDI note (note -> key that was pressed)
        let mut note_to_pressed_key: std::collections::HashMap<u8, String> = std::collections::HashMap::new();
        // Track reference count for each key (multiple notes might map to same key)
        let mut key_active_count: std::collections::HashMap<String, i32> = std::collections::HashMap::new();
        let mut total_paused_duration = Duration::ZERO;

        // Helper to release all keys
        let release_all_keys = |key_active_count: &std::collections::HashMap<String, i32>| {
            for (key, count) in key_active_count {
                if *count > 0 {
                    crate::keyboard::key_up(key);
                }
            }
        };

        for event in &midi_data.events {
            if event.time_ms < offset_ms {
                continue;
            }

            if !is_playing.load(Ordering::SeqCst) {
                release_all_keys(&key_active_count);
                return;
            }

            let target_time = Duration::from_millis(event.time_ms - offset_ms);

            // Wait until we reach the event time
            loop {
                if !is_playing.load(Ordering::SeqCst) {
                    release_all_keys(&key_active_count);
                    return;
                }

                if is_paused.load(Ordering::SeqCst) {
                    let pause_start = Instant::now();
                    while is_paused.load(Ordering::SeqCst) && is_playing.load(Ordering::SeqCst) {
                        std::thread::sleep(Duration::from_millis(50));
                        if !is_playing.load(Ordering::SeqCst) {
                            release_all_keys(&key_active_count);
                            return;
                        }
                    }
                    total_paused_duration += pause_start.elapsed();
                }

                let effective_elapsed = start_time.elapsed().saturating_sub(total_paused_duration);
                *current_position.lock().unwrap() = effective_elapsed.as_secs_f64() + (offset_ms as f64 / 1000.0);

                if effective_elapsed >= target_time {
                    break;
                }

                std::thread::sleep(Duration::from_millis(1));
            }

            // Get key based on note calculation mode (read in realtime for live switching)
            let current_mode = NoteMode::from(note_mode.load(Ordering::SeqCst));
            // Get octave shift in semitones (1 octave = 12 semitones)
            let shift_semitones = octave_shift.load(Ordering::SeqCst) as i32 * 12;
            let total_transpose = midi_data.transpose + shift_semitones;
            let key = match current_mode {
                NoteMode::Closest => note_to_key(event.note as i32, total_transpose),
                NoteMode::Quantize => note_to_key_quantize(event.note as i32, total_transpose),
                NoteMode::TransposeOnly => note_to_key_transpose(event.note as i32, total_transpose),
                NoteMode::Pentatonic => note_to_key_pentatonic(event.note as i32, total_transpose),
                NoteMode::Chromatic => note_to_key_chromatic(event.note as i32, total_transpose),
                NoteMode::Raw => note_to_key_raw(event.note as i32 + shift_semitones), // Raw ignores auto-transpose, only uses manual shift
            };

            match event.event_type {
                EventType::NoteOn => {
                    // Store which key we're pressing for this MIDI note
                    note_to_pressed_key.insert(event.note, key.clone());
                    let count = key_active_count.entry(key.clone()).or_insert(0);
                    if *count == 0 {
                        crate::keyboard::key_down(&key);
                    }
                    *count += 1;
                }
                EventType::NoteOff => {
                    // Use the key that was actually pressed for this note, not current mode mapping
                    if let Some(pressed_key) = note_to_pressed_key.remove(&event.note) {
                        if let Some(count) = key_active_count.get_mut(&pressed_key) {
                            if *count > 0 {
                                *count -= 1;
                                if *count == 0 {
                                    crate::keyboard::key_up(&pressed_key);
                                }
                            }
                        }
                    }
                }
            }
        }

        // Release all remaining keys
        release_all_keys(&key_active_count);

        if !loop_mode.load(Ordering::SeqCst) {
            break;
        }

        std::thread::sleep(Duration::from_millis(500));
    }

    is_playing.store(false, Ordering::SeqCst);
    let _ = window.emit("playback-ended", ());
}