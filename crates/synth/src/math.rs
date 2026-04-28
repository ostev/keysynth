pub trait Lerp {
    fn lerp(self, other: Self, weight: Self) -> Self;
}

impl Lerp for f32 {
    fn lerp(self, other: Self, weight: Self) -> Self {
        self + weight * (other - self)
    }
}
