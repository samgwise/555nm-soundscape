use rodiox::source::diffusion::Diffusion;
use std::f32;
use std::fmt::Debug;
use std::sync::{Arc, Mutex};
use std::time::Duration;
use rodio::Device;
use rodio::Sample;
use rodio::Sink;
use rodio::Source;

pub struct DiffusionSink {
    sink: Sink,
    positions: Arc<Mutex<SoundPositions>>,
}

struct SoundPositions {
    emitter_position: [f32; 3],
    speakers: Vec<[f32; 3]>,
}

impl DiffusionSink {
    /// Builds a new `DiffusionSink`.
    #[inline]
    pub fn new(
        device: &Device, emitter_position: [f32; 3], speakers: Vec<[f32; 3]>
    ) -> DiffusionSink {
        DiffusionSink {
            sink: Sink::new(device),
            positions: Arc::new(Mutex::new(SoundPositions {
                emitter_position,
                speakers,
            })),
        }
    }

    /// Sets the position of the sound emitter in 3 dimensional space.
    pub fn set_emitter_position(&mut self, pos: [f32; 3]) {
        self.positions.lock().unwrap().emitter_position = pos;
    }

    /// Sets the position of the left ear in 3 dimensional space.
    pub fn set_speaker_positions(&mut self, pos: Vec<[f32; 3]>) {
        self.positions.lock().unwrap().speakers = pos;
    }

    /// Appends a sound to the queue of sounds to play.
    #[inline]
    pub fn append<S>(&self, source: S)
    where
        S: Source + Send + 'static,
        S::Item: Sample + Send + Debug,
    {
        let positions = self.positions.clone();
        let pos_lock = self.positions.lock().unwrap();
        let source = Diffusion::new(
            source,
            pos_lock.emitter_position,
            &pos_lock.speakers,
        ).periodic_access(Duration::from_millis(10), move |i| {
            let pos = positions.lock().unwrap();
            i.set_positions(pos.emitter_position, &pos.speakers);
        });
        self.sink.append(source);
    }

    // Gets the volume of the sound.
    ///
    /// The value `1.0` is the "normal" volume (unfiltered input). Any value other than 1.0 will
    /// multiply each sample by this value.
    #[inline]
    pub fn volume(&self) -> f32 {
        self.sink.volume()
    }

    /// Changes the volume of the sound.
    ///
    /// The value `1.0` is the "normal" volume (unfiltered input). Any value other than 1.0 will
    /// multiply each sample by this value.
    #[inline]
    pub fn set_volume(&mut self, value: f32) {
        self.sink.set_volume(value);
    }

    /// Resumes playback of a paused sound.
    ///
    /// No effect if not paused.
    #[inline]
    pub fn play(&self) {
        self.sink.play();
    }

    /// Pauses playback of this sink.
    ///
    /// No effect if already paused.
    ///
    /// A paused sound can be resumed with `play()`.
    pub fn pause(&self) {
        self.sink.pause();
    }

    /// Gets if a sound is paused
    ///
    /// Sounds can be paused and resumed using pause() and play(). This gets if a sound is paused.
    pub fn is_paused(&self) -> bool {
        self.sink.is_paused()
    }

    /// Destroys the sink without stopping the sounds that are still playing.
    #[inline]
    pub fn detach(self) {
        self.sink.detach();
    }

    /// Sleeps the current thread until the sound ends.
    #[inline]
    pub fn sleep_until_end(&self) {
        self.sink.sleep_until_end();
    }

    /// Returns true if this sink has no more sounds to play.
    #[inline]
    pub fn empty(&self) -> bool {
        self.sink.empty()
    }
}
