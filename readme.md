# 555nm Soundscape #
TODO: [![Build Status](https://travis-ci.org/samgwise/555nm-soundscape.svg?branch=master)](https://travis-ci.org/samgwise/555nm-soundscape)

Audio player for the 555nm Vivid installation.

## Overview ##

Much work is still to be done...

The proof of concept app provides cross-platform audio playback using [rodio](https://crates.io/crates/rodio) and basic IP control with OSC messages using [rosc](https://crates.io/crates/rosc).
I2C interface is planned but yet to be implemented. The [i2cdev](https://crates.io/crates/i2cdev) looks like a good foundation for SMBus communication to the lighting rig. The semantics of this interface are yet to be designed.

## Usage ##

Not yet implemented

## Setup instructions ##

### ALSA ###
If you are running on a Linux based system, such as a Raspberry Pi, you will need ALSA as it is required for audio playback.

ALSA is not installed on many Debian based Linux distributions.
Installation can be achieved through a package manager, such as aptitude.
To do so you will need an active internet connection.
If you haven't recently updated your package list, first run:
```
sudo apt update
```
When your package list is updated, install ALSA with:
```
sudo apt install libasound2-dev
```

Following this install you need to create a configuration for ALSA.
Create your config file `/etc/asound.conf`.
You can use any editor you like, such as vim, but nano will be used here for simplicity.
```
sudo nano /etc/asound.conf
```
This will open a new file for editing which will be created when saved.
There are two entries which we must define, the default pcm and ctl entities.
```
pcm.!default {
  type hw
  card 0
}


ctl.!default {
  type hw
  card 0
}
```
This will define the default sound output as the analogue out.
Restart the device with:
```
sudo reboot
```

After restarting the device you can test the audio setup with the following:
```
speaker-test --test wav --channels 2
```

#### Troubleshooting notes ####
If you are hearing a lot of noise from a Raspberry Pi's audio out, try adding:
```
audio_pwm_mode=2
```
To `/boot/config.txt` and restart the Pi.

### Building the application ###
Firstly install the rust toolchain on the device:
```
curl https://sh.rustup.rs -sSf | sh
```

Download the source code by cloning this git repo:
```
git clone <git repo url>
```

Then in the root directory use Rust's cargo utility to build our project and it's dependencies:
```
cargo build --release
```
The executable can then be found in `target/release/555nm-soundscape`


### 4ch audio ###
Multiple options are available for achieving 4 channels of audio output from a Raspberry Pi.
- Expansion hat (No options easily available currently)
  - Custom GPIO arrangement (Unsearched)
- External USB sound cards with multiple output channels
- A Combination of built-in analogue output with stereo USB sound card with ALSA configuration
- Video/Audio splitter from HDMI (No obvious choices)

#### Multi-channel ALSA configuration
Update your ALSA configuration to group two devices into a single output.
See: https://alsa.opensrc.org/TwoCardsAsOne
