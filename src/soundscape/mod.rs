
use rodio;
use rodio::Sink;
use rodio::Source;
use rodio::Endpoint;

use config::SoundResource;

pub struct SoundSource {
    pub channel: Sink,
    pub min_threshold: f32,
    pub max_threshold: f32,
}

pub fn resource_to_sound_source(res: &SoundResource, endpoint: &Endpoint) -> SoundSource {
    SoundSource {
        channel: Sink::new(endpoint),
        min_threshold: res.min_threshold,
        max_threshold: res.max_threshold,
    }
}
