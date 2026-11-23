pub fn gain(volume: u8) -> f32 {
    let v = volume.min(100) as f32 / 100.0;
    v * v
}

/// Gain with a soft knee above about -1.4 dBFS, so a loud step does not hard-clip.
pub fn amplify(sample: i16, gain: f32) -> i16 {
    if gain == 0.0 {
        return 0;
    }
    let x = sample as f32 * gain;
    let knee = 28_000.0;
    if x.abs() <= knee {
        return x.clamp(-32767.0, 32767.0) as i16;
    }
    let sign = if x < 0.0 { -1.0 } else { 1.0 };
    let over = x.abs() - knee;
    let shaped = knee + 4_767.0 * (1.0 - (-over / 8_000.0).exp());
    (sign * shaped.min(32_767.0)) as i16
}

pub fn apply(samples: &mut [i16], volume: u8) {
    match volume.min(100) {
        100 => {}
        0 => samples.fill(0),
        v => {
            let g = gain(v);
            for s in samples.iter_mut() {
                *s = (*s as f32 * g) as i16;
            }
        }
    }
}
