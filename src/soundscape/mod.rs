
use rodiox::diffusion_sink::DiffusionSink;
use rodio::Device;

use bspline;

use config;
use config::SoundResource;

use std::cmp::Ordering;

pub struct SoundSource {
    pub channel:        DiffusionSink,
    pub min_threshold:  f32,
    pub max_threshold:  f32,
    pub gain:           f32,
    pub volume:         f32,
    pub volume_step:    f32,
    pub volume_updates: u32,
    pub fade_out_steps: u32,
    pub fade_in_steps:  u32,
    pub is_live:        bool, // Is the suound within threshhold bounds
}

pub fn resource_to_sound_source(res: &SoundResource, output_device: &Device, speakers: &Vec<[f32; 3]>) -> SoundSource {
    let position = match res.position {
        Some (pos)  => pos,
        None        => [0.0, 1.0, 1.0],
    };

    let fade_in = match res.fade_in_steps {
        Some (duration)     => duration,
        None                => 500,
    };

    let fade_out = match res.fade_out_steps {
        Some (duration)     => duration,
        None                => 500,
    };

    SoundSource {
        channel:        DiffusionSink::new(output_device, position, speakers.to_vec()),
        min_threshold:  res.min_threshold,
        max_threshold:  res.max_threshold,
        gain:           res.gain,
        volume:         0f32,
        volume_step:    0.01,
        volume_updates: 0,
        fade_in_steps:  fade_in,
        fade_out_steps: fade_out,
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

// Structure
pub struct Structure {
    pub spline:         bspline::BSpline<f32>,
    pub duration:       f32,
    pub step_t:         f32,
    pub step:           f32,
}

pub fn structure_from_scene(scene: &config::Scene) -> Structure {
    let spline      = config::to_b_spline(&scene.structure);
    let duration    = scene.cycle_duration_ms as f32;
    let step_t      = spline.knot_domain().1 / duration;
    Structure {
        spline:     spline,
        duration:   duration,
        step_t:     step_t,
        step:       0.0,
    }
}

// Command
#[derive(Copy, Clone, Eq, PartialEq)]
pub enum Cmd {
    Play,
    Load (usize),
    LoadBackground,
    Retire,
}


#[derive(Copy, Clone, Eq, PartialEq)]
pub struct FutureCmd {
    pub command: Cmd,
    pub at_tick: u64,
}

pub fn play_at(tick: u64) -> FutureCmd {
    FutureCmd { command: Cmd::Play, at_tick: tick }
}

pub fn load_at(scene_index: usize, tick: u64) -> FutureCmd {
    FutureCmd { command: Cmd::Load(scene_index), at_tick: tick }
}

pub fn load_background() -> FutureCmd {
    FutureCmd { command: Cmd::LoadBackground, at_tick: 0 }
}

pub fn retire_at(tick: u64) -> FutureCmd {
    FutureCmd { command: Cmd::Retire, at_tick: tick }
}

// Explicitly implement the trait so the queue becomes a min-heap instead of a max-heap.
impl Ord for FutureCmd {
    fn cmp(&self, other: &FutureCmd) -> Ordering {
        // Notice that the we flip the ordering on costs.
        // In case of a tie we compare positions - this step is necessary
        // to make implementations of PartialEq and Ord consistent.
        other.at_tick.cmp(&self.at_tick)
            .then_with(|| self.at_tick.cmp(&other.at_tick))
    }
}

// PartialOrd needs to be implemented as well.
impl PartialOrd for FutureCmd {
    fn partial_cmp(&self, other: &FutureCmd) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

pub fn is_cmd_now(command: Option<&FutureCmd>, ticks: &u64) -> bool {
    match command {
        Some(cmd)   => ticks >= &cmd.at_tick,
        None        => false,
    }
}
