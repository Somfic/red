#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Blinker {
    #[default]
    None,
    Left,
    Right,
}
