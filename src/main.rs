use alsa::pcm::{Access, Format, Frames, HwParams, PCM};
use alsa::{Direction, ValueOr};
use clap::Parser;
use std::sync::Arc;
use std::time::{Duration, Instant};
use std::{thread, time};

#[derive(Parser, Debug)]
struct CommandLineArgs {
    #[clap(short, long)]
    output: String,

    #[clap(short, long, default_value_t = 44100)]
    rate: u32,

    #[clap(short, long, default_value_t = 2)]
    num_periods: u32,

    #[clap(short, long, default_value_t = 384)]
    period_size: i64,
}

fn main() {
    let args = Arc::new(CommandLineArgs::parse());
    let pcm = PCM::new(&args.output, Direction::Playback, false).unwrap();
    let hwp = HwParams::any(&pcm).unwrap();
    hwp.set_channels(2).unwrap();
    hwp.set_period_size(args.period_size as Frames, alsa::ValueOr::Nearest)
        .unwrap();
    hwp.set_periods(args.num_periods, alsa::ValueOr::Nearest)
        .unwrap();
    hwp.set_rate(args.rate, ValueOr::Nearest).unwrap();
    hwp.set_format(Format::s32()).unwrap();
    hwp.set_access(Access::RWInterleaved).unwrap();
    pcm.hw_params(&hwp).unwrap();

    let swp = pcm.sw_params_current().unwrap();
    swp.set_start_threshold((args.period_size * args.num_periods as i64) as Frames)
        .unwrap();
    pcm.sw_params(&swp).unwrap();
    
    let dev = pcm.io_i32();
    let output = dev.unwrap();
    let mut buf = vec![0i32; args.period_size as usize];

    // play silence first
    output.writei(&buf[..]).unwrap();
    pcm.start().unwrap();

    let mut start_time = Instant::now();
    let mut earlier = Instant::now();

    loop {
        let later = Instant::now();
        let duration = later.duration_since(earlier).as_millis();

        if duration > 1000 {
            earlier = later;
            buf[0] = i32::MAX;
            buf[1] = -i32::MAX;
            buf[2] = i32::MAX;
        }
        else {
            buf[0] = 0;
            buf[1] = 0;
            buf[2] = 0;
        }

        if later.duration_since(start_time).as_secs() > 5 {
           // thread::sleep(time::Duration::from_millis(50));
            start_time = later;
        }

        match output.writei(&buf[..]) {
            Err(e) => {
                println!("recover from buffer underrun");
                pcm.try_recover(e, true).unwrap();
            },
            Ok(_num_frames) => {}
        }
    }
}
