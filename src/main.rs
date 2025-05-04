extern crate anyhow;
extern crate cpal;

use chrono::{Timelike, Utc};
use ringbuf::traits::Consumer;
use tokio::time::{interval, sleep_until, Duration, Instant, MissedTickBehavior};
use frequency_weightings::generate_weightings;

mod umik_1;
mod dsp;
mod frequency_weightings;

mod domain {
    pub mod types;
}

#[tokio::main]
async fn main() {
    log4rs::init_file("./log.yml", Default::default()).unwrap();
    log::info!("Noisy USB Sound Level Meter");

    if let Some((_stream, mut consumer, mic_cal_data)) = umik_1::connect().await {
        let weightings = generate_weightings((48000.0 * 2.0) as usize , 48000.0, mic_cal_data);

        let now = Utc::now();
        let next_second = now.with_nanosecond(0).unwrap() + chrono::Duration::seconds(1);
        let diff = (next_second - now).num_nanoseconds().unwrap() as u64;
        sleep_until(Instant::now() + Duration::from_nanos(diff + 1_000_000_000)).await;

        let mut leq_interval = interval(Duration::from_secs(1));
        leq_interval.set_missed_tick_behavior(MissedTickBehavior::Skip);

        loop {
                leq_interval.tick().await;
                let now = Utc::now();
                //println!("{}", now);
                let data: Vec<f32> = consumer.pop_iter().take(48_000).collect();
                match dsp::process_raw_data(data, weightings.clone()){
                    Ok((spl_z, spl_a, spl_c)) => {
                        log::info!("{} dB(Z) | {} dB(A) | {} dB(C)|", spl_z, spl_a, spl_c);
                        println!("{:?} : {} dB(Z) | {} dB(A) | {} dB(C)| ", now, spl_z, spl_a  ,spl_c);
                    }

                    Err(e) => println!("{:?}", e),
                };

            }

    } else {
        // Failed to connect
        eprintln!("Failed to connect to the audio device.");
    }
}
