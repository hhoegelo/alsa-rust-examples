use alsa::pcm::{Access, Format, Frames, HwParams, PCM};
use alsa::{Direction, ValueOr};
use clap::Parser;
use std::cmp;
use std::sync::mpsc::{channel, Receiver, Sender};
use std::sync::Arc;
use std::time::{Duration, Instant};

#[derive(Debug, Clone)]
struct PeakNotFoundError;

type PeakFindResult = std::result::Result<(Instant, usize), PeakNotFoundError>;

#[derive(Parser, Debug)]
struct CommandLineArgs {
    #[clap(short, long)]
    input: String,

    #[clap(short, long)]
    output: String,

    #[clap(short, long, default_value_t = 44100)]
    rate: u32,

    #[clap(short, long, default_value_t = 2)]
    num_periods: u32,

    #[clap(short, long, default_value_t = 1024)]
    period_size: i64,

    #[clap(short, long, default_value_t = 100)]
    tries: u32,
}

fn main() {
    let args = Arc::new(CommandLineArgs::parse());

    let mut min = u128::MAX;
    let mut max = u128::MIN;

    for _ in 0..args.tries {
        let (tx, rx) = channel();
        let args_for_sender = args.clone();
        let args_for_receiver = args.clone();
        let player = std::thread::spawn(move || play_impulse(args_for_sender.as_ref(), rx));
        let finder = std::thread::spawn(move || find_impulse(args_for_receiver.as_ref(), tx));
        let playback_time = player.join().unwrap();

        match finder.join().unwrap() {
            Ok(detection_result) => {
                let channels = 2;
                let block_receive_time = detection_result.0;
                let sample_offset_in_block = detection_result.1;
                let frame_offset_in_block = sample_offset_in_block / channels;
                let micro = 1000 * 1000;
                let micros_in_block = micro * frame_offset_in_block / args.rate as usize;
                let detection_time =
                    block_receive_time + Duration::from_micros(micros_in_block as u64);
                let latency = detection_time - playback_time;
                min = cmp::min(min, latency.as_micros());
                max = cmp::max(max, latency.as_micros());
            }
            Err(e) => {
                println!("Error: {:?}", e);
                return;
            }
        }
    }

    println!(
        "Measured Out/In latency: {} - {} = {} us",
        max,
        min,
        max - min
    );
}

fn play_impulse(args: &CommandLineArgs, rx: Receiver<bool>) -> Instant {
    let pcm = PCM::new(&args.output, Direction::Playback, false).unwrap();
    setup_pcm(&pcm, args);
    let output = pcm.io_i32().unwrap();
    let mut buf = vec![0i32; args.period_size as usize];

    // wait for capturing device to start
    rx.recv().unwrap();

    // play silence first
    let mut played_frames = output.writei(&buf[..]).unwrap();
    pcm.start().unwrap();

    let max_play_time = 5; // seconds

    for v in buf.iter_mut() {
        *v = i32::MAX;
    }

    // play the rect
    played_frames += output.writei(&buf[..]).unwrap();
    let play_time = Instant::now();

    // wait for detection or timeout
    while played_frames < args.rate as usize * max_play_time {
        played_frames += output.writei(&buf[..]).unwrap();
        match rx.try_recv() {
            Ok(false) => break,
            Ok(true) => continue,
            Err(_) => continue,
        }
    }

    return play_time;
}

fn find_impulse(args: &CommandLineArgs, tx: Sender<bool>) -> PeakFindResult {
    let pcm = PCM::new(&args.input, Direction::Capture, false).unwrap();
    setup_pcm(&pcm, args);
    let input = pcm.io_i32().unwrap();
    let mut buf = vec![0i32; args.period_size as usize];

    pcm.start().unwrap();

    let mut total = 0;
    let max_search_time = 2; // seconds

    loop {
        let num_frames_read = input.readi(&mut buf).unwrap();
        let num_samples_read = num_frames_read * 2;
        let sample_to_find = std::i32::MAX / 50;

        if total == 0 && num_frames_read > 0 {
            tx.send(true).unwrap();
        }
        for (idx, val) in buf[0..num_samples_read].iter().enumerate() {
            // println!("found {} looking for {}", (*val).abs(), sample_to_find);
            if (*val).abs() > sample_to_find {
                tx.send(false).unwrap();
                return Ok((Instant::now(), idx));
            }
        }

        total = total + num_frames_read;

        if total > args.rate as usize * max_search_time {
            return Err(PeakNotFoundError);
        }
    }
}

fn setup_pcm(pcm: &PCM, args: &CommandLineArgs) {
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
}
