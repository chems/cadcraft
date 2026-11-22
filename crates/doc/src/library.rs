//! CADCraft's own standard libraries: linetypes and hatch patterns.
//!
//! These definitions were written for CADCraft (CC0 / public domain as part of this codebase).
//! The names follow the industry-common names users expect (DASHED, CENTER, ANSI31…); the
//! dash and spacing values are our own.

use crate::{DashElement, Drawing, Linetype};

pub mod files;

/// The standard linetype library, defined in inches. Load it into a drawing with
/// [`standard_linetypes_for`], which scales it for metric drawings.
pub fn standard_linetypes() -> Vec<Linetype> {
    let s = |name: &str, desc: &str, d: &[f64]| Linetype::simple(name, desc, d);
    vec![
        s("BORDER", "Border __ __ . __ __ . __ __ .", &[0.5, -0.25, 0.5, -0.25, 0.0, -0.25]),
        s("CENTER", "Center ____ _ ____ _ ____", &[1.25, -0.25, 0.25, -0.25]),
        s("CENTER2", "Center (.5x) ___ _ ___ _", &[0.75, -0.125, 0.125, -0.125]),
        s("CENTERX2", "Center (2x) ________  __  ________", &[2.5, -0.5, 0.5, -0.5]),
        s("DASHDOT", "Dash dot __ . __ . __ .", &[0.5, -0.25, 0.0, -0.25]),
        s("DASHDOT2", "Dash dot (.5x) _._._._.", &[0.25, -0.125, 0.0, -0.125]),
        s("DASHED", "Dashed __ __ __ __", &[0.5, -0.25]),
        s("DASHED2", "Dashed (.5x) _ _ _ _ _", &[0.25, -0.125]),
        s("DASHEDX2", "Dashed (2x) ____  ____  ____", &[1.0, -0.5]),
        s("DIVIDE", "Divide ____ . . ____ . .", &[0.5, -0.25, 0.0, -0.25, 0.0, -0.25]),
        s("DOT", "Dot . . . . . . . .", &[0.0, -0.25]),
        s("DOT2", "Dot (.5x) ........", &[0.0, -0.125]),
        s("HIDDEN", "Hidden __ __ __ __", &[0.25, -0.125]),
        s("HIDDEN2", "Hidden (.5x) _ _ _ _ _", &[0.125, -0.0625]),
        s("HIDDENX2", "Hidden (2x) ____ ____ ____", &[0.5, -0.25]),
        s("PHANTOM", "Phantom ______  __  __  ______", &[1.25, -0.25, 0.25, -0.25, 0.25, -0.25]),
        s("PHANTOM2", "Phantom (.5x) ___ _ _ ___", &[0.625, -0.125, 0.125, -0.125, 0.125, -0.125]),
        Linetype {
            name: "FENCELINE1".into(),
            description: "Fenceline circle ----0-----0----".into(),
            pattern: vec![
                DashElement::dash(0.25),
                // The text goes where its element ends: centred between the two gaps.
                DashElement {
                    text: Some("o".into()),
                    style: Some("Standard".into()),
                    scale: 0.1,
                    offset: cadcraft_geom::Vec2::new(-0.035, -0.035),
                    ..DashElement::dash(-0.1)
                },
                DashElement::dash(-0.1),
                DashElement::dash(0.5),
            ],
        },
        s("ZIGZAG", "Zig zag /\\/\\/\\/\\/", &[0.0001, -0.2, 0.0001, -0.2]),
        // Building services runs, each named by a short label set in the line (the letters most
        // drawings use; projects with other codes load their own .lin file).
        labelled("WATER_COLD", "Cold water ---- CW ----", "CW"),
        labelled("WATER_HOT", "Hot water ---- HW ----", "HW"),
        labelled("WATER_HOT_RETURN", "Hot water return ---- HWR ----", "HWR"),
        labelled("HEATING_FLOW", "Heating flow ---- HF ----", "HF"),
        labelled("HEATING_RETURN", "Heating return ---- HR ----", "HR"),
        labelled("WASTE_WATER", "Waste water ---- WW ----", "WW"),
        labelled("RAIN_WATER", "Rainwater ---- RW ----", "RW"),
        labelled("GAS_LINE", "Gas ---- G ----", "G"),
        labelled("POWER_LINE", "Power cable ---- E ----", "E"),
        labelled("DATA_LINE", "Data and telecom ---- D ----", "D"),
        s("PROPERTY_LINE", "Property line ______ . . ______ . .", &[1.0, -0.125, 0.0, -0.125, 0.0, -0.125]),
    ]
}

/// Height of the label of a [`labelled`] linetype (inches; 2.54 mm in a metric drawing).
const LABEL_HEIGHT: f64 = 0.1;
/// Length of the dash between two labels (inches).
const LABEL_DASH: f64 = 0.6;

/// A line with `label` set in it at every repeat: a dash, then a gap as wide as the label and a
/// margin on each side, the label centred in the gap and upright along the line.
fn labelled(name: &str, description: &str, label: &str) -> Linetype {
    // The drafting font advances about 0.9 x the height per character.
    let chars = f64::from(u32::try_from(label.chars().count()).unwrap_or(u32::MAX));
    let half = (0.9 * LABEL_HEIGHT * chars) / 2.0;
    let margin = LABEL_HEIGHT * 0.6;
    Linetype {
        name: name.into(),
        description: description.into(),
        pattern: vec![
            DashElement::dash(LABEL_DASH),
            // The label goes where its element ends, the middle of the gap, moved back half its
            // width and down half its height so it sits centred on the line.
            DashElement {
                text: Some(label.into()),
                style: Some("Standard".into()),
                scale: LABEL_HEIGHT,
                offset: cadcraft_geom::Vec2::new(-half, -LABEL_HEIGHT / 2.0),
                ..DashElement::dash(-(half + margin))
            },
            DashElement::dash(-(half + margin)),
        ],
    }
}

/// `p`, drawn in millimetres, as a library pattern (defined in inches like the others), so a
/// metric drawing draws it at its real size at hatch scale 1.
fn real_size_mm(p: HatchPattern) -> HatchPattern {
    scaled_pattern(p, 1.0 / 25.4)
}

/// The standard linetypes as loaded into `d`. In a metric drawing (`MEASUREMENT` 1) every length
/// (dashes, gaps, text and shape size, offsets) is multiplied by 25.4, so the patterns measure
/// in millimetres what they measure in inches in an imperial drawing (DASHED: 12.7 on, 6.35
/// off) and LTSCALE 1 suits both, as CAD programs load ISO-scaled definitions for metric
/// drawings. Definitions already in a drawing are not changed.
pub fn standard_linetypes_for(d: &Drawing) -> Vec<Linetype> {
    let lib = standard_linetypes();
    if d.header.i64("MEASUREMENT", 0) != 1 {
        return lib;
    }
    lib.into_iter().map(|lt| scaled(lt, 25.4)).collect()
}

/// `lt` with every length multiplied by `k`.
fn scaled(mut lt: Linetype, k: f64) -> Linetype {
    for e in &mut lt.pattern {
        e.length *= k;
        e.offset = e.offset * k;
        if e.text.is_some() || e.shape.is_some() {
            e.scale *= k;
        }
    }
    lt
}

/// A hatch pattern line family: angle (degrees), origin, offset (shift along, spacing), dashes.
#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct PatternLine {
    pub angle: f64,
    pub origin: (f64, f64),
    pub delta: (f64, f64),
    pub dashes: Vec<f64>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct HatchPattern {
    pub name: &'static str,
    pub description: &'static str,
    pub lines: Vec<PatternLine>,
}

fn pl(angle: f64, origin: (f64, f64), delta: (f64, f64), dashes: &[f64]) -> PatternLine {
    PatternLine { angle, origin, delta, dashes: dashes.to_vec() }
}

/// The standard hatch pattern library.
pub fn standard_patterns() -> Vec<HatchPattern> {
    vec![
        HatchPattern { name: "SOLID", description: "Solid fill", lines: vec![] },
        HatchPattern { name: "ANSI31", description: "Iron, brick, stone masonry", lines: vec![pl(45.0, (0.0, 0.0), (0.0, 0.125), &[])] },
        HatchPattern {
            name: "ANSI32",
            description: "Steel",
            lines: vec![pl(45.0, (0.0, 0.0), (0.0, 0.375), &[]), pl(45.0, (0.176777, 0.0), (0.0, 0.375), &[])],
        },
        HatchPattern {
            name: "ANSI33",
            description: "Bronze, brass, copper",
            lines: vec![pl(45.0, (0.0, 0.0), (0.0, 0.25), &[]), pl(45.0, (0.176777, 0.0), (0.0, 0.25), &[0.125, -0.0625])],
        },
        HatchPattern {
            name: "ANSI34",
            description: "Plastic, rubber",
            lines: (0..4).map(|i| pl(45.0, (0.176777 * f64::from(i) / 2.0, 0.0), (0.0, 0.75), &[])).collect(),
        },
        HatchPattern {
            name: "ANSI35",
            description: "Fire brick, refractory",
            lines: vec![pl(45.0, (0.0, 0.0), (0.0, 0.25), &[]), pl(45.0, (0.176777, 0.0), (0.0, 0.25), &[0.3125, -0.0625, 0.0, -0.0625])],
        },
        HatchPattern {
            name: "ANSI36",
            description: "Marble, slate, glass",
            lines: vec![pl(45.0, (0.0, 0.0), (0.21875, 0.125), &[0.3125, -0.0625, 0.0, -0.0625])],
        },
        HatchPattern {
            name: "ANSI37",
            description: "Lead, zinc, magnesium",
            lines: vec![pl(45.0, (0.0, 0.0), (0.0, 0.125), &[]), pl(135.0, (0.0, 0.0), (0.0, 0.125), &[])],
        },
        HatchPattern {
            name: "ANSI38",
            description: "Aluminum",
            lines: vec![pl(45.0, (0.0, 0.0), (0.0, 0.125), &[]), pl(135.0, (0.0, 0.0), (0.25, 0.125), &[0.3125, -0.1875])],
        },
        HatchPattern { name: "LINE", description: "Parallel horizontal lines", lines: vec![pl(0.0, (0.0, 0.0), (0.0, 0.125), &[])] },
        HatchPattern {
            name: "NET",
            description: "Horizontal / vertical grid",
            lines: vec![pl(0.0, (0.0, 0.0), (0.0, 0.125), &[]), pl(90.0, (0.0, 0.0), (0.0, 0.125), &[])],
        },
        HatchPattern {
            name: "NET3",
            description: "Network pattern 0-60-120",
            lines: vec![pl(0.0, (0.0, 0.0), (0.0, 0.125), &[]), pl(60.0, (0.0, 0.0), (0.0, 0.125), &[]), pl(120.0, (0.0, 0.0), (0.0, 0.125), &[])],
        },
        HatchPattern { name: "DOTS", description: "A series of dots", lines: vec![pl(0.0, (0.0, 0.0), (0.03125, 0.0625), &[0.0, -0.0625])] },
        HatchPattern { name: "DASH", description: "Dashed lines", lines: vec![pl(0.0, (0.0, 0.0), (0.125, 0.125), &[0.125, -0.125])] },
        HatchPattern {
            name: "BRICK",
            description: "Brick or masonry-type surface",
            lines: vec![
                // Running bond: courses 0.25 high, bricks 0.5 long. Each vertical family draws the
                // joints of every other course, 0.5 apart; the second is offset half a brick.
                pl(0.0, (0.0, 0.0), (0.0, 0.25), &[]),
                pl(90.0, (0.0, 0.0), (0.0, 0.5), &[0.25, -0.25]),
                pl(90.0, (0.25, 0.0), (0.0, 0.5), &[-0.25, 0.25]),
            ],
        },
        HatchPattern {
            name: "SQUARE",
            description: "Small aligned squares",
            lines: vec![pl(0.0, (0.0, 0.0), (0.0, 0.125), &[0.125, -0.125]), pl(90.0, (0.0, 0.0), (0.0, 0.125), &[0.125, -0.125])],
        },
        HatchPattern {
            name: "CROSS",
            description: "A series of crosses",
            lines: vec![pl(0.0, (0.0, 0.0), (0.25, 0.25), &[0.125, -0.375]), pl(90.0, (0.0625, -0.0625), (0.25, 0.25), &[0.125, -0.375])],
        },
        HatchPattern {
            name: "EARTH",
            description: "Earth or ground (subterranean)",
            lines: vec![
                pl(0.0, (0.0, 0.0), (0.25, 0.25), &[0.25, -0.25]),
                pl(0.0, (0.0, 0.09375), (0.25, 0.25), &[0.25, -0.25]),
                pl(0.0, (0.0, 0.1875), (0.25, 0.25), &[0.25, -0.25]),
                pl(90.0, (0.03125, 0.21875), (0.25, 0.25), &[0.25, -0.25]),
                pl(90.0, (0.125, 0.21875), (0.25, 0.25), &[0.25, -0.25]),
                pl(90.0, (0.21875, 0.21875), (0.25, 0.25), &[0.25, -0.25]),
            ],
        },
        HatchPattern {
            name: "GRASS",
            description: "Grass area",
            lines: vec![
                pl(90.0, (0.0, 0.0), (std::f64::consts::FRAC_1_SQRT_2, std::f64::consts::FRAC_1_SQRT_2), &[0.1875, -1.226]),
                pl(45.0, (0.0, 0.0), (0.0, 1.0), &[0.1875, -0.8125]),
                pl(135.0, (0.0, 0.0), (0.0, 1.0), &[0.1875, -0.8125]),
            ],
        },
        HatchPattern {
            name: "HONEY",
            description: "Honeycomb pattern",
            lines: vec![
                pl(0.0, (0.0, 0.0), (0.1875, 0.108253), &[0.125, -0.25]),
                pl(120.0, (0.0, 0.0), (0.1875, 0.108253), &[0.125, -0.25]),
                pl(60.0, (0.125, 0.0), (0.1875, 0.108253), &[0.125, -0.25]),
            ],
        },
        HatchPattern {
            name: "ZIGZAG",
            description: "Staircase effect",
            lines: vec![pl(0.0, (0.0, 0.0), (0.125, 0.125), &[0.125, -0.125]), pl(90.0, (0.125, 0.0), (0.125, 0.125), &[0.125, -0.125])],
        },
        // Drawn at the scale of the pattern CAD programs ship under this name (about 16x
        // coarser than first drawn here: AR-CONC hatches came out hundreds of times too dense).
        scaled_pattern(
            HatchPattern {
                name: "AR-CONC",
                description: "Random dot and stone pattern",
                lines: vec![
                    pl(50.0, (0.0, 0.0), (0.4, 0.25), &[0.15, -0.6]),
                    pl(355.0, (0.0, 0.0), (0.27, 0.3), &[0.12, -0.5]),
                    pl(100.0, (0.12, 0.05), (0.35, 0.33), &[0.1, -0.55]),
                    pl(0.0, (0.07, 0.02), (0.21, 0.17), &[0.0, -0.4]),
                ],
            },
            16.0,
        ),
        // Drawn at the scale of the pattern CAD programs ship under this name (about 16x
        // coarser than first drawn here: AR-CONC hatches came out hundreds of times too dense).
        scaled_pattern(
            HatchPattern {
                name: "AR-SAND",
                description: "Random dot pattern",
                lines: vec![
                    pl(37.5, (0.0, 0.0), (0.112, 0.0912), &[0.0, -0.22, 0.0, -0.14]),
                    pl(7.5, (0.0, 0.0), (0.157, 0.0724), &[0.0, -0.19, 0.0, -0.31]),
                    pl(-32.5, (0.0, 0.0), (0.091, 0.1312), &[0.0, -0.27, 0.0, -0.11]),
                ],
            },
            16.0,
        ),
        // Outdoor surfaces at their real size in a metric drawing (hatch scale 1), drawn for
        // CADCraft: slabs, block pavers, timber decking, gravel, planting and water.
        real_size_mm(HatchPattern {
            name: "PAVING_SLABS",
            description: "Square paving slabs 400 x 400",
            lines: vec![pl(0.0, (0.0, 0.0), (0.0, 400.0), &[]), pl(90.0, (0.0, 0.0), (0.0, 400.0), &[])],
        }),
        real_size_mm(HatchPattern {
            name: "PAVERS",
            description: "Block pavers 200 x 100, stretcher bond",
            lines: vec![
                pl(0.0, (0.0, 0.0), (0.0, 100.0), &[]),
                pl(90.0, (0.0, 0.0), (0.0, 200.0), &[100.0, -100.0]),
                pl(90.0, (100.0, 0.0), (0.0, 200.0), &[-100.0, 100.0]),
            ],
        }),
        real_size_mm(HatchPattern {
            name: "DECKING",
            description: "Timber decking, boards 145 wide, staggered butt joints",
            lines: vec![
                pl(0.0, (0.0, 0.0), (0.0, 145.0), &[]),
                pl(90.0, (0.0, 0.0), (0.0, 2400.0), &[145.0, -290.0]),
                pl(90.0, (800.0, 145.0), (0.0, 2400.0), &[145.0, -290.0]),
                pl(90.0, (1600.0, 290.0), (0.0, 2400.0), &[145.0, -290.0]),
            ],
        }),
        real_size_mm(HatchPattern {
            name: "GRAVEL_BED",
            description: "Gravel or chippings",
            lines: vec![
                pl(0.0, (0.0, 0.0), (51.0, 69.0), &[0.0, -123.0]),
                pl(60.0, (15.0, 21.0), (87.0, 57.0), &[12.0, -159.0]),
                pl(120.0, (33.0, 9.0), (69.0, 93.0), &[9.0, -141.0]),
                pl(15.0, (6.0, 39.0), (111.0, 87.0), &[0.0, -183.0]),
            ],
        }),
        real_size_mm(HatchPattern {
            name: "PLANTING",
            description: "Planting bed, shrubs and ground cover",
            // Small crosses on a 300 grid: each horizontal dash is crossed at its middle.
            lines: vec![pl(0.0, (0.0, 0.0), (0.0, 300.0), &[60.0, -240.0]), pl(90.0, (30.0, -30.0), (0.0, 300.0), &[60.0, -240.0])],
        }),
        real_size_mm(HatchPattern {
            name: "WATER_SURFACE",
            description: "Pond or pool water surface",
            lines: vec![pl(0.0, (0.0, 0.0), (225.0, 150.0), &[300.0, -150.0, 100.0, -150.0])],
        }),
    ]
}

pub fn pattern(name: &str) -> Option<HatchPattern> {
    standard_patterns().into_iter().find(|p| p.name.eq_ignore_ascii_case(name))
}

/// The library pattern `name` as drawn in `d`. In a metric drawing (`MEASUREMENT` 1) every
/// length is multiplied by 25.4, as CAD programs use ISO-scaled pattern definitions there (as
/// for linetypes, see [`standard_linetypes_for`]): a hatch at scale 1 measures in millimetres
/// what it measures in inches in an imperial drawing.
pub fn pattern_for(d: &Drawing, name: &str) -> Option<HatchPattern> {
    let p = pattern(name)?;
    Some(if d.header.i64("MEASUREMENT", 0) == 1 { scaled_pattern(p, 25.4) } else { p })
}

/// `p` with every length (origins, offsets, dashes) multiplied by `k`.
fn scaled_pattern(mut p: HatchPattern, k: f64) -> HatchPattern {
    for l in &mut p.lines {
        l.origin = (l.origin.0 * k, l.origin.1 * k);
        l.delta = (l.delta.0 * k, l.delta.1 * k);
        for d in &mut l.dashes {
            *d *= k;
        }
    }
    p
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A name the library defined twice would load whichever came first: every linetype and
    /// pattern name is defined once (in any case).
    #[test]
    fn library_names_are_unique() {
        let mut names: Vec<String> = standard_linetypes().into_iter().map(|l| l.name.to_ascii_uppercase()).collect();
        let n = names.len();
        names.sort();
        names.dedup();
        assert_eq!(names.len(), n, "a linetype name is defined twice");
        let mut names: Vec<String> = standard_patterns().into_iter().map(|p| p.name.to_ascii_uppercase()).collect();
        let n = names.len();
        names.sort();
        names.dedup();
        assert_eq!(names.len(), n, "a pattern name is defined twice");
    }

    /// A labelled run leaves a gap wider than its label, and the label sits inside the gap, so
    /// the text never runs over the dashes either side.
    #[test]
    fn a_labelled_linetype_leaves_room_for_its_label() {
        for lt in standard_linetypes().iter().filter(|l| l.pattern.iter().any(|e| e.text.is_some())) {
            let gap: f64 = lt.pattern.iter().filter(|e| e.length < 0.0).map(|e| -e.length).sum();
            let Some(text) = lt.pattern.iter().find(|e| e.text.is_some()) else { continue };
            let width = 0.9 * text.scale * text.text.as_deref().map_or(0.0, |t| t.chars().count() as f64);
            assert!(gap > width, "{}: gap {gap} for a label {width} wide", lt.name);
        }
    }

    /// Outdoor surfaces are drawn at their real size in a metric drawing: decking boards 145 mm
    /// apart at hatch scale 1.
    #[test]
    fn outdoor_patterns_are_real_size_in_metric_drawings() {
        let deck = pattern_for(&Drawing::new_metric(), "DECKING").map(|p| p.lines[0].delta.1);
        assert!(deck.is_some_and(|d| (d - 145.0).abs() < 1e-9), "{deck:?}");
    }

    #[test]
    fn metric_drawings_draw_iso_scaled_patterns() {
        let imp = pattern_for(&Drawing::new_imperial(), "ANSI31").map(|p| p.lines[0].delta.1);
        let met = pattern_for(&Drawing::new_metric(), "ansi31").map(|p| p.lines[0].delta.1);
        assert_eq!(imp, Some(0.125));
        assert!(met.is_some_and(|m| (m - 0.125 * 25.4).abs() < 1e-12), "{met:?}");
        assert!(pattern_for(&Drawing::new_metric(), "NOPE").is_none());
    }
}
