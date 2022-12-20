use alsa::pcm::{Access, Format, Frames, HwParams, PCM};
use alsa::{Direction, ValueOr};
use clap::Parser;
use std::sync::Arc;

#[derive(Parser, Debug)]
struct CommandLineArgs {
    #[clap(short, long)]
    output: String,

    #[clap(short, long, default_value_t = 44100)]
    rate: usize,

    #[clap(short, long, default_value_t = 2)]
    num_periods: usize,

    #[clap(short, long, default_value_t = 384)]
    period_size: usize,
}

fn main() {
    let args = Arc::new(CommandLineArgs::parse());
    let pcm = PCM::new(&args.output, Direction::Playback, false).unwrap();
    let hwp = HwParams::any(&pcm).unwrap();
    hwp.set_channels(2).unwrap();
    hwp.set_period_size(args.period_size as Frames, alsa::ValueOr::Nearest)
        .unwrap();
    hwp.set_periods(args.num_periods as u32, alsa::ValueOr::Nearest)
        .unwrap();
    hwp.set_rate(args.rate as u32, ValueOr::Nearest).unwrap();
    hwp.set_format(Format::s32()).unwrap();
    hwp.set_access(Access::RWInterleaved).unwrap();
    pcm.hw_params(&hwp).unwrap();

    let swp = pcm.sw_params_current().unwrap();
    swp.set_start_threshold((args.period_size * args.num_periods) as Frames)
        .unwrap();
    pcm.sw_params(&swp).unwrap();

    let output = pcm.io_i32().unwrap();
    let buf = vec![0i32; args.period_size];

    output.writei(&buf).unwrap();
    pcm.start().unwrap();

    let mut last_delay = pcm.delay().unwrap_or(0);

    loop {
        let delay = pcm.delay().unwrap_or(0);

        if delay != last_delay {
            println!("Delay changed: {}", delay);
            last_delay = delay;
        }

        match output.writei(&buf) {
            Err(e) => {
                println!("recover from buffer underrun");
                pcm.try_recover(e, true).unwrap();
            }
            Ok(num_frames) => {
                if num_frames == 0 {
                    println!("have to re-start");
                    pcm.start().unwrap();
                }
            }
        }
    }
}
