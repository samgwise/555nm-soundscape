use cgmath::{InnerSpace, Point3};
use rodio::source::ChannelVolume;
use std::fmt::Debug;
use std::time::Duration;
use rodio::Sample;
use rodio::Source;

/// Combines channels in input into a single mono source, then plays that mono sound
/// to each channel at the volume given for that channel.
#[derive(Clone, Debug)]
pub struct Diffusion<I>
where
    I: Source,
    I::Item: Sample + Debug,
{
    input: ChannelVolume<I>,
}

impl<I> Diffusion<I>
where
    I: Source,
    I::Item: Sample + Debug,
{
    pub fn new(
        input: I, emitter_position: [f32; 3], speakers: &Vec<[f32; 3]>,
    ) -> Diffusion<I>
    where
        I: Source,
        I::Item: Sample,
    {
        let mut levels :Vec<f32> = Vec::with_capacity(speakers.len());
        for i in 0..speakers.len() {
            levels.push(0.0)
        }

        let mut ret = Diffusion {
            input: ChannelVolume::new(input, levels),
        };

        ret.set_positions(emitter_position, speakers);
        ret
    }

    /// Sets the position of the emitter and ears in the 3D world.
    pub fn set_positions(
        &mut self, emitter_pos: [f32; 3], speakers: &Vec<[f32; 3]>,
    ) {
        let emitter_position = Point3::from(emitter_pos);
        let mut channel_count = 0;
        for speaker_pos in speakers {
            let speaker_position = Point3::new(speaker_pos[0], speaker_pos[1], speaker_pos[2]);

            let distance = (speaker_position - emitter_position).magnitude();

            let amplitude = (1.0 / (distance * 2.0)).abs();

            if amplitude > 1.0 {
                println!("Warning: Amplitude {} is greater than 1.0 for source at {:?} and speaker at {:?}!", amplitude, emitter_position, speaker_pos);
            }

            self.input.set_volume(channel_count, amplitude);
            channel_count += 1;
        }
    }
}

impl<I> Iterator for Diffusion<I>
where
    I: Source,
    I::Item: Sample + Debug,
{
    type Item = I::Item;

    #[inline]
    fn next(&mut self) -> Option<I::Item> {
        self.input.next()
    }

    #[inline]
    fn size_hint(&self) -> (usize, Option<usize>) {
        self.input.size_hint()
    }
}

impl<I> ExactSizeIterator for Diffusion<I>
where
    I: Source + ExactSizeIterator,
    I::Item: Sample + Debug,
{
}

impl<I> Source for Diffusion<I>
where
    I: Source,
    I::Item: Sample + Debug,
{
    #[inline]
    fn current_frame_len(&self) -> Option<usize> {
        self.input.current_frame_len()
    }

    #[inline]
    fn channels(&self) -> u16 {
        self.input.channels()
    }

    #[inline]
    fn sample_rate(&self) -> u32 {
        self.input.sample_rate()
    }

    #[inline]
    fn total_duration(&self) -> Option<Duration> {
        self.input.total_duration()
    }
}
