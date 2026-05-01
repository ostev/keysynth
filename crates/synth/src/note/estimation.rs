use crate::math::Lerp;

pub struct PitchEstimator<'a> {
    samples: &'a [f32],
    /// We store the reciprocal of the length so we don't
    /// have to compute the divsion later
    length_reciprocal: f32,
}

impl<'a> PitchEstimator<'a> {
    /// Creates a new pitch estimator. Will return `None` if the buffer
    /// is empty.
    pub const fn new(samples: &'a [f32]) -> Option<Self> {
        if samples.len() > 0 {
            Some(Self {
                samples,
                length_reciprocal: (samples.len() as f32).recip(),
            })
        } else {
            None
        }
    }

    pub fn estimate_frequency(&self, sample_rate: f32, min_frequency: f32) -> f32 {
        let period = self.estimate_period(max_lag_for_min_frequency(sample_rate, min_frequency));

        sample_rate / period
    }

    pub fn estimate_period(&self, max_lag: usize) -> f32 {
        // Whether the graph has passed its first maximum stationary point
        let mut has_passed_first_max = false;
        // The current gradient of the graph. Used to determine whether the
        // graph has passed its first maximum stationary point (gradient changes
        // sign from positive to negative).
        let mut gradient: f32 = 0.0;
        // The smallest ASDF found so far. Used to determine
        // the global minimum within the sample window.
        let mut smallest_asdf = f32::INFINITY;
        // The lag value whether the smallest ASDF occured.
        let mut lag_for_smallest_asdf: usize = 0;
        // The calculated ASDF of the previous sample. Used to calculate the
        // gradient
        let mut previous_asdf: f32 = 0.0;

        // The period is the minimum of the ASDF graph within the sample window
        // *after* the first maximum stationary point.
        for lag in 0..max_lag {
            let asdf = self.asdf(lag);

            if has_passed_first_max || {
                let new_gradient = asdf - previous_asdf;

                // Has the sign of the gradient changed from positive to negative?
                // If so, we just crossed a maximum stationary point in between this
                // sample and the previous.
                has_passed_first_max =
                    gradient.is_sign_positive() && new_gradient.is_sign_negative();

                gradient = new_gradient;

                has_passed_first_max
            } {
                if asdf < smallest_asdf {
                    smallest_asdf = asdf;
                    lag_for_smallest_asdf = lag;
                }
            }

            previous_asdf = asdf;
        }

        let period = lag_for_smallest_asdf;
        // let asdf_before = self.asdf(period - 2);
        // let asdf_after = self.asdf(period + 2);

        // let p1 = (asdf_before - min_asdf);
        // let p2 = asdf_after - min_asdf;
        // let weight = if p1 > p2 { p2 / p1 } else { p1 / p2 };
        // let interpolated_period = (if p1 > p2 { (period + 2) } else { period - 2 } as f32).lerp(
        //     if p1 > p2 { (period - 2) } else { period + 2 } as f32,
        //     weight,
        // );
        // interpolated_period
        period as f32
    }

    /// Computes the average magnitude squared function of the sample buffer
    /// for a given lag value.
    fn asdf(&self, lag: usize) -> f32 {
        let start = lag / 2 + 1;

        let total: f32 = self
            .samples
            .iter()
            .enumerate()
            .skip(start)
            .take_while(|(index, _)| *index < self.samples.len() - lag / 2)
            .map(|(index, _)| self.samples[index + lag / 2] - self.samples[index - lag / 2])
            .map(|difference| difference * difference)
            .sum();

        self.length_reciprocal * total
    }
}

pub const fn max_lag_for_min_frequency(sample_rate: f32, frequency: f32) -> usize {
    (sample_rate / frequency) as usize
}

#[cfg(test)]
mod tests {
    use crate::{
        DEFAULT_SAMPLE_RATE, Synth,
        note::{self, Note},
        wavetable::Wavetable,
    };

    use super::*;

    #[test]
    fn identifies_middle_c() {
        let sin_wavetable: Wavetable<2048> = Wavetable::from_fn(libm::sinf);
        let mut synth: Synth<'_, 1, 2048> =
            Synth::new(DEFAULT_SAMPLE_RATE as f32, [&sin_wavetable, &sin_wavetable]);

        synth
            .note_on(note::Event {
                note: Note::C4,
                timestamp: 0,
            })
            .unwrap();

        let samples: [f32; 4096] = synth.sample_many();
        let pitch_estimator = PitchEstimator::new(&samples).unwrap();

        let period = pitch_estimator.estimate_period(1000);

        assert!(((DEFAULT_SAMPLE_RATE as f32 / Note::C4.frequency) - period).abs() < 1.0);
    }
}
