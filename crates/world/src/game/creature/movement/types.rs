//! Movement types - generator types, speeds, and spline flags

/// Movement generator type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum MovementGeneratorType {
    Idle = 0,
    Random = 1,
    Waypoint = 2,
    Follow = 3,
    Point = 5,
    Chase = 8,
    Fleeing = 9,
    Home = 10,
    Effect = 11,
    Taxi = 12,
}

/// Movement state for packet generation
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum MoveType {
    Walk,
    Run,
    RunBack,
    Swim,
    SwimBack,
    TurnRate,
    Flight,
    FlightBack,
}

/// Movement speeds
#[derive(Debug, Clone)]
pub struct MovementSpeeds {
    pub walk: f32,
    pub run: f32,
    pub run_back: f32,
    pub swim: f32,
    pub swim_back: f32,
    pub turn_rate: f32,
    pub flight: f32,
    pub flight_back: f32,
}

impl Default for MovementSpeeds {
    fn default() -> Self {
        Self {
            walk: 2.5,
            run: 7.0,
            run_back: 4.5,
            swim: 4.722222,
            swim_back: 2.5,
            turn_rate: 3.141594,
            flight: 7.0,
            flight_back: 4.5,
        }
    }
}

/// Spline flags for movement packets
pub mod spline_flags {
    pub const DONE: u32 = 0x00000001;
    pub const FALLING: u32 = 0x00000002;
    pub const FLYING: u32 = 0x00000200;
    pub const NO_SPLINE: u32 = 0x00000400;
    pub const WALKMODE: u32 = 0x00000100;
    pub const RUNMODE: u32 = 0x00000000;
    pub const CATMULLROM: u32 = 0x00100000;
}
