use crate::driver::Blinker;

#[derive(Clone, Copy, Default)]
pub enum YieldResolver {
    #[default]
    RightOfWay,
}

#[derive(Clone, Copy, Debug)]
pub enum TurnType {
    Straight,
    Right(f32),
    Left(f32),
}

impl TurnType {
    pub fn cross(&self) -> f32 {
        match self {
            TurnType::Straight => 0.0,
            TurnType::Right(cross) => *cross,
            TurnType::Left(cross) => *cross,
        }
    }

    pub fn blinker(&self) -> Blinker {
        match self {
            TurnType::Straight => Blinker::None,
            TurnType::Right(cross) => {
                if cross.abs() > 0.3 {
                    Blinker::Right
                } else {
                    Blinker::None
                }
            }
            TurnType::Left(cross) => {
                if cross.abs() > 0.3 {
                    Blinker::Left
                } else {
                    Blinker::None
                }
            }
        }
    }
}
