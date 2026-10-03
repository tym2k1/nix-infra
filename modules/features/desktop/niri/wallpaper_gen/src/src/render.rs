use crate::turtle::{Point, Tree};
use std::fmt::Write;
use std::fs;
use std::path::Path;

pub struct Canvas {
    pub width: f64,
    pub height: f64,
}

/// A rendered instance of a generated tree.
///
/// The tree geometry is unique to this instance, but the complete
/// tree is represented by exactly one SVG path object.
///
/// Scale, rotation and opacity are properties of the whole tree.
pub struct TreeInstance {
    pub tree: Tree,
    pub position: Point,
    pub scale: f64,
    pub rotation: f64,
    pub opacity: f64,
}

pub fn render(
    path: impl AsRef<Path>,
    canvas: Canvas,
    trees: &[TreeInstance],
    offset_x: f64,
    offset_y: f64,
) -> Result<(), std::io::Error> {
    let mut svg = String::new();

    writeln!(
        svg,
        r##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 {} {}">"##,
        canvas.width,
        canvas.height,
    )
    .unwrap();

    writeln!(
        svg,
        r##"<rect width="100%" height="100%" fill="#f4f1e8"/>"##
    )
    .unwrap();

    //
    // Every tree instance becomes exactly ONE SVG path object.
    //
    // This is important: opacity is applied once to the complete
    // tree rather than separately to its branches.
    //
    for tree in trees {
        render_tree(
            &mut svg,
            tree,
            offset_x,
            offset_y,
        );
    }

    svg.push_str("</svg>\n");

    fs::write(path, svg)
}

fn render_tree(
    svg: &mut String,
    instance: &TreeInstance,
    offset_x: f64,
    offset_y: f64,
) {
    //
    // One stroke width for the entire tree.
    //
    // Projection scale controls the apparent size of the tree.
    //
    let stroke_width = instance.opacity;
    let opacity = instance.opacity;

    write!(
        svg,
        r##"<path fill="none"
           stroke="#111111"
           stroke-width="{:.2}"
           stroke-linecap="round"
           stroke-linejoin="round"
           opacity="1.0"
           d=""##,
        stroke_width,
    )
    .unwrap();

    //
    // All branches are encoded as subpaths inside this ONE SVG path.
    //
    for branch in &instance.tree.branches {
        if branch.points.len() < 2 {
            continue;
        }

        let first = transform_point(
            branch.points[0],
            instance,
            offset_x,
            offset_y,
        );

        write!(
            svg,
            " M{:.2},{:.2}",
            first.x,
            first.y,
        )
        .unwrap();

        for point in &branch.points[1..] {
            let point = transform_point(
                *point,
                instance,
                offset_x,
                offset_y,
            );

            write!(
                svg,
                " L{:.2},{:.2}",
                point.x,
                point.y,
            )
            .unwrap();
        }
    }

    svg.push_str(r#""/>"#);
    svg.push('\n');
}

fn transform_point(
    point: Point,
    instance: &TreeInstance,
    offset_x: f64,
    offset_y: f64,
) -> Point {
    let cos = instance.rotation.cos();
    let sin = instance.rotation.sin();

    //
    // Apply projection scale in canonical tree-local coordinates.
    //
    let x = point.x * instance.scale;
    let y = point.y * instance.scale;

    //
    // Rotate the tree around its root so that it follows the
    // ground tangent.
    //
    let rotated_x =
        x * cos - y * sin;

    let rotated_y =
        x * sin + y * cos;

    Point {
        x: rotated_x
            + instance.position.x
            + offset_x,

        y: rotated_y
            + instance.position.y
            + offset_y,
    }
}
