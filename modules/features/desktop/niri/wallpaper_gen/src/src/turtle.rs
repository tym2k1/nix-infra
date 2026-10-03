use std::f64::consts::PI;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Point {
    pub x: f64,
    pub y: f64,
}

/// One continuous drawable branch.
///
/// Points are kept in canonical local coordinates. The renderer
/// transforms the entire tree when creating its forest instance.
#[derive(Debug)]
pub struct Branch {
    pub points: Vec<Point>,
}

/// A canonical tree generated from one expanded L-system program.
///
/// Every call to `interpret()` creates a new Tree. This allows each
/// forest tree to have unique L-system geometry while still keeping
/// the resulting tree as one renderable SVG object.
#[derive(Debug)]
pub struct Tree {
    pub branches: Vec<Branch>,
}

#[derive(Debug, Clone, Copy)]
struct Turtle {
    position: Point,
    angle: f64,
}

#[derive(Debug, Clone, Copy)]
struct State {
    turtle: Turtle,
}

/// Interpret an expanded L-system program into one canonical tree.
///
/// The tree is generated in local coordinates:
///
/// - root = (0, 0)
/// - initial direction = upward
/// - positive local Y points downward
///
/// Each continuous run of forward movement becomes one `Branch`.
///
/// The branches are only a geometry representation. The renderer
/// combines all of them into ONE SVG `<path>` object.
pub fn interpret(
    program: &str,
    step: f64,
    angle_degrees: f64,
) -> Tree {
    let angle = angle_degrees * PI / 180.0;

    let mut turtle = Turtle {
        position: Point {
            x: 0.0,
            y: 0.0,
        },
        angle: PI / 2.0,
    };

    let mut stack: Vec<State> = Vec::new();

    let mut branches = Vec::<Branch>::new();

    //
    // The current continuous branch.
    //
    // It begins at the current turtle position and receives forward
    // movement until a branch operation causes continuity to stop.
    //
    let mut current_branch = vec![turtle.position];

    for symbol in program.chars() {
        match symbol {
            'F' => {
                let end = Point {
                    x: turtle.position.x
                        + turtle.angle.cos() * step,

                    y: turtle.position.y
                        - turtle.angle.sin() * step,
                };

                turtle.position = end;

                current_branch.push(end);
            }

            '+' => {
                turtle.angle += angle;
            }

            '-' => {
                turtle.angle -= angle;
            }

            '[' => {
                //
                // The current branch ends at the branch point.
                //
                finish_branch(
                    &mut branches,
                    &mut current_branch,
                );

                //
                // Save the parent turtle state.
                //
                stack.push(State { turtle });

                //
                // The child branch starts at this branch point.
                //
                current_branch = vec![turtle.position];
            }

            ']' => {
                //
                // Finish the child branch.
                //
                finish_branch(
                    &mut branches,
                    &mut current_branch,
                );

                //
                // Restore the parent branch point and begin a new
                // continuous parent branch there.
                //
                if let Some(state) = stack.pop() {
                    turtle = state.turtle;

                    current_branch = vec![turtle.position];
                }
            }

            _ => {}
        }
    }

    //
    // Finish the final branch.
    //
    finish_branch(
        &mut branches,
        &mut current_branch,
    );

    Tree { branches }
}

fn finish_branch(
    branches: &mut Vec<Branch>,
    current_branch: &mut Vec<Point>,
) {
    //
    // A drawable branch needs at least two points.
    //
    if current_branch.len() >= 2 {
        branches.push(Branch {
            points: std::mem::take(current_branch),
        });
    } else {
        //
        // Preserve the current position so a future branch can still
        // begin from the correct location.
        //
        current_branch.truncate(1);
    }
}
