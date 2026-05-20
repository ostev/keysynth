pub struct Model {}

impl Model {
    pub fn view<'a>(
        &'a self,
        v: &'a embedded_gui::view::Factory,
    ) -> embedded_gui::view::View<'a, Self::Target, Self::FocusKey> {
        todo!()
    }
}
