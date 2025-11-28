use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU8, AtomicI8, Ordering};
use std::time::Instant;
use tauri::Window;
use serde::{Serialize, Deserialize};

use crate::midi::{NoteMode, KeyMode};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlaybackState {
    pub is_playing: bool,
    pub is_paused: bool,
    pub current_position: f64,
    pub total_duration: f64,
    pub current_file: Option<String>,
    pub loop_mode: bool,
    pub note_mode: NoteMode,
    pub key_mode: KeyMode,
    pub octave_shift: i8,
}

pub struct AppState {
    is_playing: Arc<AtomicBool>,
    is_paused: Arc<AtomicBool>,
    loop_mode: Arc<AtomicBool>,
    note_mode: Arc<AtomicU8>,
    key_mode: Arc<AtomicU8>,
    octave_shift: Arc<AtomicI8>,
    current_position: Arc<std::sync::Mutex<f64>>,
    total_duration: Arc<std::sync::Mutex<f64>>,
    current_file: Arc<std::sync::Mutex<Option<String>>>,
    playback_start: Arc<std::sync::Mutex<Option<Instant>>>,
    midi_data: Arc<std::sync::Mutex<Option<crate::midi::MidiData>>>,
    seek_offset: Arc<std::sync::Mutex<f64>>,
}

impl AppState {
    pub fn new() -> Self {
        AppState {
            is_playing: Arc::new(AtomicBool::new(false)),
            is_paused: Arc::new(AtomicBool::new(false)),
            loop_mode: Arc::new(AtomicBool::new(false)),
            note_mode: Arc::new(AtomicU8::new(NoteMode::Closest as u8)),
            key_mode: Arc::new(AtomicU8::new(KeyMode::Keys21 as u8)),
            octave_shift: Arc::new(AtomicI8::new(0)),
            current_position: Arc::new(std::sync::Mutex::new(0.0)),
            total_duration: Arc::new(std::sync::Mutex::new(0.0)),
            current_file: Arc::new(std::sync::Mutex::new(None)),
            playback_start: Arc::new(std::sync::Mutex::new(None)),
            midi_data: Arc::new(std::sync::Mutex::new(None)),
            seek_offset: Arc::new(std::sync::Mutex::new(0.0)),
        }
    }

    pub fn load_midi(&mut self, path: &str) -> Result<(), String> {
        let midi_data = crate::midi::load_midi(path)?;

        *self.total_duration.lock().unwrap() = midi_data.duration;
        *self.current_file.lock().unwrap() = Some(path.to_string());
        *self.midi_data.lock().unwrap() = Some(midi_data);
        // Reset seek offset and position for new song
        *self.seek_offset.lock().unwrap() = 0.0;
        *self.current_position.lock().unwrap() = 0.0;

        Ok(())
    }

    pub fn start_playback(&mut self, window: Window) -> Result<(), String> {
        if let Some(midi_data) = self.midi_data.lock().unwrap().clone() {
            self.is_playing.store(true, Ordering::SeqCst);
            self.is_paused.store(false, Ordering::SeqCst);
            let offset = *self.seek_offset.lock().unwrap();
            *self.playback_start.lock().unwrap() = Some(Instant::now());
            *self.current_position.lock().unwrap() = offset;

            // Clone Arc references for the thread
            let is_playing = Arc::clone(&self.is_playing);
            let is_paused = Arc::clone(&self.is_paused);
            let loop_mode = Arc::clone(&self.loop_mode);
            let note_mode = Arc::clone(&self.note_mode);
            let key_mode = Arc::clone(&self.key_mode);
            let octave_shift = Arc::clone(&self.octave_shift);
            let current_position = Arc::clone(&self.current_position);
            let seek_offset = Arc::clone(&self.seek_offset);

            std::thread::spawn(move || {
                crate::midi::play_midi(
                    midi_data,
                    is_playing,
                    is_paused,
                    loop_mode,
                    note_mode,
                    key_mode,
                    octave_shift,
                    current_position,
                    seek_offset,
                    window
                );
            });

            Ok(())
        } else {
            Err("No MIDI file loaded".to_string())
        }
    }

    pub fn set_note_mode(&mut self, mode: NoteMode) {
        self.note_mode.store(mode as u8, Ordering::SeqCst);
    }

    pub fn get_note_mode(&self) -> NoteMode {
        NoteMode::from(self.note_mode.load(Ordering::SeqCst))
    }

    pub fn set_key_mode(&mut self, mode: KeyMode) {
        self.key_mode.store(mode as u8, Ordering::SeqCst);
    }

    pub fn get_key_mode(&self) -> KeyMode {
        KeyMode::from(self.key_mode.load(Ordering::SeqCst))
    }

    pub fn set_octave_shift(&mut self, shift: i8) {
        // Clamp to -2 to +2 octaves
        let clamped = shift.clamp(-2, 2);
        self.octave_shift.store(clamped, Ordering::SeqCst);
    }

    pub fn get_octave_shift(&self) -> i8 {
        self.octave_shift.load(Ordering::SeqCst)
    }

    pub fn toggle_pause(&mut self) {
        if self.is_playing.load(Ordering::SeqCst) {
            let paused = !self.is_paused.load(Ordering::SeqCst);
            self.is_paused.store(paused, Ordering::SeqCst);
        }
    }

    pub fn stop_playback(&mut self) {
        self.is_playing.store(false, Ordering::SeqCst);
        self.is_paused.store(false, Ordering::SeqCst);
        *self.current_position.lock().unwrap() = 0.0;
        *self.playback_start.lock().unwrap() = None;

        // Wait for the playback thread to detect the stop flag and clean up
        std::thread::sleep(std::time::Duration::from_millis(100));
    }

    pub fn set_loop_mode(&mut self, enabled: bool) {
        self.loop_mode.store(enabled, Ordering::SeqCst);
    }

    pub fn seek(&mut self, position: f64, window: Window) -> Result<(), String> {
        if self.is_playing.load(Ordering::SeqCst) {
            // Store the seek position
            *self.seek_offset.lock().unwrap() = position;

            // Restart playback from the new position
            self.stop_playback();
            self.start_playback(window)?;
        } else {
            // Just set the position if not playing
            *self.current_position.lock().unwrap() = position;
            *self.seek_offset.lock().unwrap() = position;
        }
        Ok(())
    }

    pub fn get_playback_state(&self) -> PlaybackState {
        let mut position = *self.current_position.lock().unwrap();

        // Update position based on playback time if playing
        if self.is_playing.load(Ordering::SeqCst) && !self.is_paused.load(Ordering::SeqCst) {
            if let Some(start_time) = *self.playback_start.lock().unwrap() {
                position = start_time.elapsed().as_secs_f64();
            }
        }

        PlaybackState {
            is_playing: self.is_playing.load(Ordering::SeqCst),
            is_paused: self.is_paused.load(Ordering::SeqCst),
            current_position: position,
            total_duration: *self.total_duration.lock().unwrap(),
            current_file: self.current_file.lock().unwrap().clone(),
            loop_mode: self.loop_mode.load(Ordering::SeqCst),
            note_mode: self.get_note_mode(),
            key_mode: self.get_key_mode(),
            octave_shift: self.get_octave_shift(),
        }
    }
}