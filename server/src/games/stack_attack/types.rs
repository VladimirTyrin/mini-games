use crate::proto::stack_attack::HorizontalDirection as ProtoDirection;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Point {
    pub x: i32,
    pub y: i32,
}

impl Point {
    pub fn new(x: i32, y: i32) -> Self {
        Self { x, y }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FieldSize {
    pub width: u32,
    pub height: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HorizontalDirection {
    Left,
    Right,
}

impl HorizontalDirection {
    pub fn dx(&self) -> i32 {
        match self {
            HorizontalDirection::Left => -1,
            HorizontalDirection::Right => 1,
        }
    }

    pub fn from_proto(proto: ProtoDirection) -> Option<Self> {
        match proto {
            ProtoDirection::Left => Some(HorizontalDirection::Left),
            ProtoDirection::Right => Some(HorizontalDirection::Right),
            ProtoDirection::Unspecified => None,
        }
    }

    pub fn to_proto(self) -> ProtoDirection {
        match self {
            HorizontalDirection::Left => ProtoDirection::Left,
            HorizontalDirection::Right => ProtoDirection::Right,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WorkerState {
    Grounded,
    Jumping,
    Falling,
}

impl WorkerState {
    pub fn to_proto(self) -> crate::proto::stack_attack::WorkerState {
        match self {
            WorkerState::Grounded => crate::proto::stack_attack::WorkerState::Grounded,
            WorkerState::Jumping => crate::proto::stack_attack::WorkerState::Jumping,
            WorkerState::Falling => crate::proto::stack_attack::WorkerState::Falling,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CraneState {
    Moving,
    Dropping,
    Leaving,
}

impl CraneState {
    pub fn to_proto(self) -> crate::proto::stack_attack::CraneState {
        match self {
            CraneState::Moving => crate::proto::stack_attack::CraneState::Moving,
            CraneState::Dropping => crate::proto::stack_attack::CraneState::Dropping,
            CraneState::Leaving => crate::proto::stack_attack::CraneState::Leaving,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GameOverReason {
    WorkerCrushed,
    BoxesReachedCeiling,
    PlayerDisconnected,
}

impl GameOverReason {
    pub fn to_proto(self) -> crate::proto::stack_attack::StackAttackGameEndReason {
        match self {
            GameOverReason::WorkerCrushed => {
                crate::proto::stack_attack::StackAttackGameEndReason::WorkerCrushed
            }
            GameOverReason::BoxesReachedCeiling => {
                crate::proto::stack_attack::StackAttackGameEndReason::BoxesReachedCeiling
            }
            GameOverReason::PlayerDisconnected => {
                crate::proto::stack_attack::StackAttackGameEndReason::PlayerDisconnected
            }
        }
    }
}

pub enum MoveResult {
    Moved,
    Blocked,
    PushedBox(u32),
}
