#[derive(Clone, Copy)]
pub struct Vcf {
    in1: f32,
    in2: f32,
    in3: f32,
    in4: f32,

    out1: f32,
    out2: f32,
    out3: f32,
    out4: f32,
}

impl Vcf {
    pub const fn new() -> Vcf {
        Vcf {
            in1: 1.0,
            in2: 1.0,
            in3: 1.0,
            in4: 1.0,
            out1: 1.0,
            out2: 1.0,
            out3: 1.0,
            out4: 1.0,
        }
    }

    pub const fn sample(&mut self, input: f32, cutoff: f32, resonance: f32) -> f32 {
        // The following code is a translation of the Moog VCF filter
        // found at https://www.musicdsp.org/en/latest/Filters/26-moog-vcf-variation-2.html
        const IN_MULTIPLIER: f32 = 0.3;

        let f = cutoff * 1.16;
        let f_squared = f * f;
        let one_minus_f = 1.0 - f;

        let feedback = resonance * (1.0 - 1.15 * f_squared);

        let modified_input = (input - (self.in4 * feedback)) * 0.35013 * f_squared * f_squared;

        // Pole 1
        self.out1 = modified_input + IN_MULTIPLIER * self.in1 + one_minus_f * self.out1;
        self.in1 = modified_input;

        // Pole 2
        self.out2 = self.out1 + IN_MULTIPLIER * self.in2 + one_minus_f * self.out2;
        self.in2 = self.out1;

        // Pole 3
        self.out3 = self.out2 + IN_MULTIPLIER * self.in3 + one_minus_f * self.out3;
        self.in3 = self.out2;

        // Pole 4
        self.out4 = self.out3 + IN_MULTIPLIER * self.in4 + one_minus_f * self.out4;
        self.in4 = self.out3;

        self.out4
    }
}
