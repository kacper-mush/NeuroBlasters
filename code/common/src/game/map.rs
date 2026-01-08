pub use crate::protocol::MapName;
use crate::protocol::{MapDefinition, RectWall, Team};
use strum::IntoEnumIterator;

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
            MapName::Tiga => MapDefinition {
                width: 800.0,
                height: 600.0,
                walls: vec![
                    RectWall {
                        min: (46.0, 137.0).into(),
                        max: (356.0, 175.0).into(),
                    },
                    RectWall {
                        min: (478.0, 450.0).into(),
                        max: (757.0, 487.0).into(),
                    },
                    RectWall {
                        min: (387.0, 240.0).into(),
                        max: (437.0, 384.0).into(),
                    },
                    RectWall {
                        min: (52.0, 443.0).into(),
                        max: (353.0, 488.0).into(),
                    },
                    RectWall {
                        min: (511.0, 133.0).into(),
                        max: (742.0, 177.0).into(),
                    },
                ],
                spawn_points: vec![
                    (Team::Red, (69.0, 78.0).into()),
                    (Team::Red, (141.0, 77.0).into()),
                    (Team::Red, (225.0, 80.0).into()),
                    (Team::Red, (298.0, 76.0).into()),
                    (Team::Blue, (503.0, 537.0).into()),
                    (Team::Blue, (581.0, 541.0).into()),
                    (Team::Blue, (654.0, 542.0).into()),
                    (Team::Blue, (716.0, 544.0).into()),
                ],
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn map_name_next_cycles_forward() {
        let first = MapName::Basic;
        let next = first.next();
        assert_eq!(next, MapName::Loss);
    }

    #[test]
    fn map_name_prev_cycles_backward() {
        let first = MapName::Basic;
        let prev = first.prev();
        assert_eq!(prev, MapName::Tiga);
    }

    #[test]
    fn map_name_next_then_prev_returns_original() {
        let original = MapName::Basic;
        let result = original.next().prev();
        assert_eq!(original, result);
    }
}
