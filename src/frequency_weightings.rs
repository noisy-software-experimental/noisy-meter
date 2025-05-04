use crate::domain::types::{MicCalibrationData, Weightings};


pub fn generate_weightings(fft_size: usize, sample_rate: f32, mic_cal_data: MicCalibrationData) -> Weightings{
    return Weightings {
            a_weighting:  generate_a_weighting_gains(fft_size, sample_rate),
            c_weighting:  generate_c_weighting_gains(fft_size, sample_rate),
            cal_weighting: generate_cal_weighting_gains(fft_size, sample_rate, mic_cal_data)
    }
}

fn generate_a_weighting_gains(fft_size: usize, sample_rate: f32) -> Vec<f32> {
    let nyquist = sample_rate / 2.0;
    let bin_count = fft_size / 2; // For real FFT
    let mut gains = Vec::with_capacity(bin_count);

    for i in 0..bin_count {
        let f = i as f32 * nyquist / (bin_count - 1) as f32;

        let f2 = f * f;
        let ra_num = 12200.0 * 12200.0 * f2 * f2;
        let ra_den = (f2 + 20.6 * 20.6)
            * (f2 + 12200.0 * 12200.0)
            * ((f2 + 107.7 * 107.7) * (f2 + 737.9 * 737.9)).sqrt();

        let ra = ra_num / ra_den;
        let a_db = 20.0 * ra.log10() + 2.0;

        let gain = 10f32.powf(a_db / 20.0);
        gains.push(gain);
    }
    gains
}

fn generate_c_weighting_gains(fft_size: usize, sample_rate: f32) -> Vec<f32> {
    let nyquist = sample_rate / 2.0;
    let bin_count = fft_size / 2;
    let mut gains = Vec::with_capacity(bin_count);

    for i in 0..bin_count {
        let f = i as f32 * nyquist / (bin_count - 1) as f32;
        let f2 = f * f;

        let rc = (12200.0 * 12200.0 * f2) /
                 ((f2 + 20.6 * 20.6) * (f2 + 12200.0 * 12200.0));

        let c_db = 20.0 * rc.log10(); // No offset
        let gain = 10f32.powf(c_db / 20.0);
        gains.push(gain);
    }
    gains
}

fn generate_cal_weighting_gains(fft_size: usize, sample_rate: f32, mic_cal_data: MicCalibrationData) -> Vec<f32> {
    let num_bins = fft_size as u32 / 2; // Only positive frequencies
    let delta_f = sample_rate / fft_size as f32; //
    let target_freqs: Vec<f32> = (0..num_bins)
        .map(|i| i as f32 * delta_f + 0.5)
        .collect();
    let gains = mic_cal_data.interpolate(&target_freqs);
    gains
}
