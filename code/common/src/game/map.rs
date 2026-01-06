use crate::protocol::{MapDefinition, RectWall, Team};
use strum::IntoEnumIterator;
use strum_macros::EnumIter;

#[derive(EnumIter, Copy, Clone, Debug, PartialEq, Eq)]
pub enum MapName {
    Basic,
    Loss,
}

impl MapName {
    pub fn next(self) -> Self {
        let all: Vec<_> = Self::iter().collect();
        let i = all.iter().position(|&m| m == self).unwrap();
        all[(i + 1) % all.len()]
    }

    pub fn prev(self) -> Self {
        let all: Vec<_> = Self::iter().collect();
        let i = all.iter().position(|&m| m == self).unwrap();
        all[(i + all.len() - 1) % all.len()]
    }
}

impl MapDefinition {
    /// backwards compatibility
    pub fn load() -> Self {
        Self::load_name(MapName::Basic)
    }

    pub fn load_name(name: MapName) -> Self {
        match name {
            MapName::Basic => Self {
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
            },
            MapName::Loss => Self {
                width: 1080.0,
                height: 1080.0,
                walls: vec![
                    RectWall {
                        min: (86.0, 479.0).into(),
                        max: (980.0, 574.0).into(),
                    },
                    RectWall {
                        min: (475.0, 87.0).into(),
                        max: (596.0, 1006.0).into(),
                    },
                    RectWall {
                        min: (184.0, 168.0).into(),
                        max: (246.0, 482.0).into(),
                    },
                    RectWall {
                        min: (182.0, 571.0).into(),
                        max: (243.0, 901.0).into(),
                    },
                    RectWall {
                        min: (352.0, 570.0).into(),
                        max: (405.0, 900.0).into(),
                    },
                    RectWall {
                        min: (686.0, 173.0).into(),
                        max: (747.0, 478.0).into(),
                    },
                    RectWall {
                        min: (833.0, 216.0).into(),
                        max: (893.0, 480.0).into(),
                    },
                    RectWall {
                        min: (690.0, 570.0).into(),
                        max: (753.0, 904.0).into(),
                    },
                    RectWall {
                        min: (754.0, 810.0).into(),
                        max: (972.0, 866.0).into(),
                    },
                ],
                spawn_points: vec![
                    (Team::Red, (792.0, 407.0).into()),
                    (Team::Red, (790.0, 235.0).into()),
                    (Team::Red, (299.0, 627.0).into()),
                    (Team::Red, (295.0, 758.0).into()),
                    (Team::Blue, (303.0, 403.0).into()),
                    (Team::Blue, (420.0, 283.0).into()),
                    (Team::Blue, (643.0, 746.0).into()),
                    (Team::Blue, (845.0, 691.0).into()),
                ],
            },
        }
    }
}
