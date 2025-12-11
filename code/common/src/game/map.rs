use crate::protocol::{MapDefinition, RectWall, Team};

impl MapDefinition {
    // TODO: Actually implement map loading, with a way to store maps, etc.
    // maybe a shared map storage with map ids, so no need to send the whole
    // map through the net?

    /// For now loads 1 simple hard-coded map.
    pub fn load() -> Self {
        Self {
            width: 1600.0,
            height: 900.0,
            walls: vec![
                RectWall {
                    min: (410.0, 658.0).into(),
                    max: (1194.0, 720.0).into(),
                },
                RectWall {
                    min: (417.0, 173.0).into(),
                    max: (1170.0, 238.0).into(),
                },
                RectWall {
                    min: (1157.0, 386.0).into(),
                    max: (1358.0, 431.0).into(),
                },
                RectWall {
                    min: (1326.0, 527.0).into(),
                    max: (1537.0, 570.0).into(),
                },
                RectWall {
                    min: (100.0, 535.0).into(),
                    max: (321.0, 584.0).into(),
                },
                RectWall {
                    min: (259.0, 372.0).into(),
                    max: (504.0, 427.0).into(),
                },
                RectWall {
                    min: (787.0, 322.0).into(),
                    max: (828.0, 566.0).into(),
                },
            ],
            spawn_points: vec![
                (Team::Red, (460.0, 822.0).into()),
                (Team::Red, (634.0, 818.0).into()),
                (Team::Red, (851.0, 823.0).into()),
                (Team::Red, (1095.0, 823.0).into()),
                (Team::Blue, (1061.0, 77.0).into()),
                (Team::Blue, (840.0, 70.0).into()),
                (Team::Blue, (666.0, 74.0).into()),
                (Team::Blue, (479.0, 78.0).into()),
            ],
        }
    }
}
