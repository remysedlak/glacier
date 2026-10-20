use wide::f32x4;

/// Envelope Follower. smooths RMSE values over time
pub fn envelope_follower(rms: &[f32], attack: f32, release: f32) -> Vec<f32> {
    let mut smooth_vector: Vec<f32> = vec![];
    let mut previous = 0.0;
    for rms_value in rms.iter() {
        let coefficient = if *rms_value > previous {
            attack
        } else {
            release
        };
        let smoothed_rms = smooth_toward(previous, *rms_value, coefficient);
        smooth_vector.push(smoothed_rms);
        previous = smoothed_rms;
    }
    smooth_vector
}

/// Batched version of smooth_toward for exactly 4 voices at once.
/// current/target are 4 voices' values; coeff is shared across all 4.
pub fn smooth_toward_x4(current: [f32; 4], target: [f32; 4], coeff: f32) -> [f32; 4] {
    let current_v = f32x4::new(current);
    let target_v = f32x4::new(target);
    let coeff_v = f32x4::splat(coeff);

    let result = current_v + (target_v - current_v) * coeff_v;
    result.to_array()
}

/// Flush a near-zero value to exact zero, avoiding denormal float performance penalties
/// in decay-style computations (e.g. after smooth_toward approaching 0.0).
pub fn flush_denormal(x: f32) -> f32 {
    if x.abs() < 1e-15 { 0.0 } else { x }
}

/// exponential decay
pub fn smooth_toward(current: f32, target: f32, coeff: f32) -> f32 {
    current + (target - current) * coeff
}

#[cfg(test)]
mod simd_sandbox {
    use crate::window::rms;

    use super::*;
    use wide::f32x4;

    #[test]
    fn basic_simd_add() {
        let a = f32x4::new([1.0, 2.0, 3.0, 4.0]);
        let b = f32x4::new([10.0, 20.0, 30.0, 40.0]);
        let sum = a + b;
        assert_eq!(sum.to_array(), [11.0, 22.0, 33.0, 44.0]);
    }

    #[test]
    fn simd_smooth_toward() {
        // 4 voices' current_volume
        let current = f32x4::new([0.0, 0.5, 0.2, 0.9]);
        // 4 voices' target_volume
        let target = f32x4::new([1.0, 1.0, 0.0, 0.5]);
        let coeff = f32x4::splat(0.01); // same coeff for all 4 lanes

        // current + (target - current) * coeff — same formula as smooth_toward, batched
        let result = current + (target - current) * coeff;

        let expected = [
            0.0 + (1.0 - 0.0) * 0.01,
            0.5 + (1.0 - 0.5) * 0.01,
            0.2 + (0.0 - 0.2) * 0.01,
            0.9 + (0.5 - 0.9) * 0.01,
        ];
        assert_eq!(result.to_array(), expected);
    }

    #[test]
    fn zero_envelope_follower() {
        let zero_samples: Vec<f32> = [0.0_f32; 8192].to_vec();
        let result: Vec<f32> = rms(&zero_samples, 1024, 512);
        let envelope = envelope_follower(&result, 1.0, 0.01);
        let answer = [0.0_f32; 15];
        assert_eq!(envelope, answer);
    }

    #[test]
    fn smooth_toward_x4_matches_scalar() {
        let current = [0.0, 0.5, 0.2, 0.9];
        let target = [1.0, 1.0, 0.0, 0.5];
        let coeff = 0.01;

        let batched = smooth_toward_x4(current, target, coeff);

        for i in 0..4 {
            let scalar = smooth_toward(current[i], target[i], coeff);
            assert_eq!(batched[i], scalar);
        }
    }

    #[test]
    fn benchmark_smooth_toward_simd_vs_scalar() {
        const ITERATIONS: u32 = 10_000_000;

        let current = [0.0, 0.5, 0.2, 0.9];
        let target = [1.0, 1.0, 0.0, 0.5];
        let coeff = 0.01;

        // scalar: call smooth_toward 4 times per iteration
        let start = std::time::Instant::now();
        let mut scalar_result = [0.0; 4];
        for _ in 0..ITERATIONS {
            for i in 0..4 {
                scalar_result[i] = smooth_toward(current[i], target[i], coeff);
            }
        }
        let scalar_time = start.elapsed();

        // SIMD: one batched call per iteration
        let start = std::time::Instant::now();
        let mut simd_result = [0.0; 4];
        for _ in 0..ITERATIONS {
            simd_result = smooth_toward_x4(current, target, coeff);
        }
        let simd_time = start.elapsed();

        println!("scalar: {:?}", scalar_time);
        println!("simd:   {:?}", simd_time);

        // keep both results so the compiler can't optimize the loops away entirely
        assert_eq!(scalar_result, simd_result);
    }
}
