
use rodio;
use rodio::Sink;
use rodio::Source;
use rodio::Endpoint;

use config::SoundResource;

pub struct SoundSource {
    pub channel:        Sink,
    pub min_threshold:  f32,
    pub max_threshold:  f32,
    pub gain:           f32,
    pub volume:         f32,
    pub volume_step:    f32,
    pub volume_updates: u32,
    pub is_live:        bool, // Is the suound within threshhold bounds
}

pub fn resource_to_sound_source(res: &SoundResource, endpoint: &Endpoint) -> SoundSource {
    SoundSource {
        channel:        Sink::new(endpoint),
        min_threshold:  res.min_threshold,
        max_threshold:  res.max_threshold,
        gain:           res.gain,
        volume:         0f32,
        volume_step:    0.01,
        volume_updates: 0,
        is_live:        false,
    }
}

pub fn update(source: &mut SoundSource) {
    if source.volume_updates > 0 {
        source.volume           += source.volume_step;
        source.volume_updates   -= 1;
        source.channel.set_volume(source.volume)
    }
}

pub fn volume_fade(source: &mut SoundSource, volume_target: f32, steps: u32) {
    source.volume_updates = steps;
    let steps = steps as f32;
    let fade_step = (source.volume - volume_target) / steps;
    if volume_target > source.volume {
        source.volume_step = fade_step.abs()
    }
    else {
        source.volume_step = fade_step.abs() * -1f32;
    }
}
