#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Position<'s> {
    pub line: usize,
    pub column: usize,
    pub index: usize,
    pub text: &'s str,
}
