#[derive(Hash, PartialEq, Eq, Debug, Clone)]
pub struct Identifier<'a> {
    pub name: &'a str,
}

impl<'a> Identifier<'a> {
    pub fn new(name: &'a str) -> Self {
        Identifier { name }
    }
}
