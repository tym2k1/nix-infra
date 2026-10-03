mod grammar;
mod lsystem;
mod render;
mod turtle;

use clap::Parser;
use rand::rngs::StdRng;
use rand::{RngExt, SeedableRng};

use grammar::Grammar;
use render::{render, Canvas, TreeInstance};
use turtle::{Point, Tree};

#[derive(Debug, Clone, Copy)]
struct GroundSample {
    position: Point,
    tangent: f64,
}

#[derive(Debug, Clone, Copy)]
struct Projection {
    scale: f64,
    opacity: f64,
}

#[derive(Debug, Parser)]
#[command(name = "forest")]
#[command(about = "Deterministic generative L-system forests")]
struct Args {
    /// Seed used to generate the forest.
    #[arg(long, default_value = "test")]
    seed: String,

    /// L-system grammar.
    #[arg(long, default_value = "grammars/tree.toml")]
    grammar: String,

    /// Output SVG.
    #[arg(short, long, default_value = "forest.svg")]
    output: String,

    /// Canvas width.
    #[arg(long, default_value_t = 3840.0)]
    width: f64,

    /// Canvas height.
    #[arg(long, default_value_t = 2160.0)]
    height: f64,

    /// Number of trees.
    #[arg(long, default_value_t = 30)]
    trees: usize,

    /// Base tree scale.
    #[arg(long, default_value_t = 1.0)]
    scale: f64,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();

    let grammar = Grammar::from_file(&args.grammar)?;

    let seed = seed_from_string(&args.seed);
    let mut rng = StdRng::from_seed(seed);

    //
    // Generate a smooth ground line across the bottom of the canvas.
    //
    let ground = generate_ground_path(
        args.width,
        args.height,
        &mut rng,
    );

    //
    // Each tree gets its own deterministic L-system seed and therefore
    // its own geometry.
    //
    // The resulting Tree is still rendered as exactly ONE SVG path
    // object. The randomness here changes the geometry, not the SVG
    // representation.
    //
    let mut instances = Vec::<TreeInstance>::with_capacity(args.trees);

    for index in 0..args.trees {
        //
        // Give every tree its own deterministic RNG stream.
        //
        let tree_seed = rng.random::<u64>();
        let mut tree_rng = StdRng::seed_from_u64(tree_seed);

        //
        // Generate a unique tree for this instance.
        //
        let program = lsystem::expand(
            &grammar,
            &mut tree_rng,
        );

        let tree = turtle::interpret(
            &program,
            grammar.step,
            grammar.angle,
        );

        //
        // Place trees roughly evenly along the ground.
        //
        let t = if args.trees <= 1 {
            0.5
        } else {
            index as f64 / (args.trees - 1) as f64
        };

        //
        // Small positional jitter prevents perfectly mechanical
        // spacing.
        //
        let jitter = tree_rng.random_range(-0.018..0.018);
        let sample_t = (t + jitter).clamp(0.0, 1.0);

        let ground_sample = sample_ground(
            &ground,
            sample_t,
        );

        // Keep roughly the same number of trees in the foreground as the
        // forest grows. Additional trees are increasingly pushed toward
        // the background.
        //
        // 0 = foreground
        // 1 = far background
        //
        let foreground_trees = 30.0;

        let depth = {
            let u: f64 = tree_rng.random_range(0.0..1.0);

            //
            // The first `foreground_trees` worth of probability occupy
            // the foreground half of the depth range. As the forest grows,
            // the probability of landing there decreases.
            //
            let foreground_probability =
                (foreground_trees / args.trees as f64).min(1.0);

            if u < foreground_probability {
                //
                // Foreground:
                // 0.0 = closest
                // 0.5 = middle
                //
                (u / foreground_probability) * 0.5
            } else {
                //
                // Background:
                // 0.5 = middle
                // 1.0 = farthest
                //
                0.5
                    + ((u - foreground_probability)
                        / (1.0 - foreground_probability))
                        * 0.5
            }
        };

        let projection = project_tree(
            args.scale,
            depth,
        );

        let ground_offset =
            args.height * 0.40 * depth;

        let position = Point {
            x: ground_sample.position.x,
            y: ground_sample.position.y - ground_offset,
        };

        instances.push(TreeInstance {
            tree,
            position,
            scale: projection.scale,
            rotation: ground_sample.tangent,
            opacity: projection.opacity,
        });
    }

    render(
        &args.output,
        Canvas {
            width: args.width,
            height: args.height,
        },
        &instances,
        0.0,
        0.0,
    )?;

    eprintln!(
        "generated {} tree instances",
        instances.len(),
    );

    Ok(())
}

fn seed_from_string(seed: &str) -> [u8; 32] {
    *blake3::hash(seed.as_bytes()).as_bytes()
}

/// Convert a normalized depth value into visual projection properties.
///
/// Depth convention:
///
/// - 1.0 = far background
/// - 0.0 = foreground
///
/// Background trees are intentionally:
///
/// - smaller
/// - more opaque
///
/// Foreground trees are:
///
/// - larger
/// - less transparent
///
/// This gives the forest a layered atmospheric effect while keeping
/// opacity uniform across each individual tree.
fn project_tree(
    base_scale: f64,
    depth: f64,
) -> Projection {
    let depth = depth.clamp(0.1, 1.0);

    //
    // Perspective scale.
    //
    // Background trees are substantially smaller than foreground
    // trees, but never disappear completely.
    //
    let scale_factor =
        (1.00/depth) * 0.10;

    //
    // Opacity intentionally moves in the opposite direction from
    // scale.
    //
    // Background trees are more opaque, while foreground trees are
    // lighter. This keeps distant layers visually present without
    // requiring multiple strokes per tree.
    //
    let opacity =
        1.00 - depth;

    Projection {
        scale: base_scale * scale_factor,
        opacity,
    }
}

/// Generate a gently undulating ground path.
///
/// The path occupies the bottom portion of the canvas rather than
/// placing all trees at a perfectly horizontal baseline.
fn generate_ground_path<R: rand::Rng + ?Sized>(
    width: f64,
    height: f64,
    rng: &mut R,
) -> Vec<Point> {
    const SAMPLES: usize = 32;

    let mut points = Vec::with_capacity(SAMPLES);

    //
    // Keep the forest firmly at the bottom of the canvas.
    //
    let base_y = height - height * 0.025;

    //
    // The amplitude is deliberately modest. We want the ground to
    // feel organic without making the trees look like they are
    // growing on a mountain range.
    //
    let amplitude = height * 0.035;

    let phase = rng.random_range(
        0.0..std::f64::consts::TAU,
    );

    let phase2 = rng.random_range(
        0.0..std::f64::consts::TAU,
    );

    for i in 0..SAMPLES {
        let t = i as f64 / (SAMPLES - 1) as f64;

        let x = t * width;

        //
        // Two low-frequency waves create a smoother, less
        // mechanically periodic shape.
        //
        let wave1 =
            (t * std::f64::consts::TAU + phase).sin();

        let wave2 =
            (
                t
                    * std::f64::consts::TAU
                    * 2.0
                    + phase2
            )
            .sin();

        let y = base_y
            + amplitude * (wave1 * 0.7 + wave2 * 0.3);

        points.push(Point { x, y });
    }

    points
}

/// Sample the ground path at normalized position `t`.
///
/// Linear interpolation is sufficient here because the ground path
/// itself is already densely sampled. The tangent is calculated from
/// neighboring samples so trees can follow the slope of the ground.
fn sample_ground(
    points: &[Point],
    t: f64,
) -> GroundSample {
    debug_assert!(points.len() >= 2);

    let t = t.clamp(0.0, 1.0);

    let position =
        t * (points.len() - 1) as f64;

    let index = position.floor() as usize;

    let next_index =
        (index + 1).min(points.len() - 1);

    let local_t =
        position - index as f64;

    let a = points[index];
    let b = points[next_index];

    let position = Point {
        x: a.x + (b.x - a.x) * local_t,
        y: a.y + (b.y - a.y) * local_t,
    };

    //
    // Use a wider neighborhood where possible. This avoids trees
    // reacting too strongly to tiny changes in the ground curve.
    //
    let tangent_a =
        index.saturating_sub(1);

    let tangent_b =
        (index + 1).min(points.len() - 1);

    let tangent_start =
        points[tangent_a];

    let tangent_end =
        points[tangent_b];

    let dx =
        tangent_end.x - tangent_start.x;

    let dy =
        tangent_end.y - tangent_start.y;

    let tangent =
        dy.atan2(dx);

    GroundSample {
        position,
        tangent,
    }
}
