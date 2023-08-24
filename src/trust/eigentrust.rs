use ndarray::arr2;

#[test]
fn debug() {
    let a = arr2(&[[1., 0.9, 0.5], [0.8, 1., 0.2], [0.3, 0.4, 1.]]);

    println!("{:?}", a);
}

// https://docs.rs/dot/0.1.4/dot/
// https://crates.io/crates/tabbycat
// https://github.com/datproject/dat
// https://www.reddit.com/r/rust/comments/g3ub83/is_anyone_using_rust_analyzer_editing_on_remote/

const THRESHOLD: f64 = 1e-9;
const MAX_ITERATIONS: usize = 1000;

fn normalize(matrix: &Vec<Vec<f64>>) -> Vec<Vec<f64>> {
    let n = matrix.len();
    let mut normalized = vec![vec![0.0; n]; n];

    for i in 0..n {
        let sum: f64 = matrix[i].iter().sum();
        for j in 0..n {
            normalized[i][j] = if sum == 0.0 { 0.0 } else { matrix[i][j] / sum };
        }
    }
    normalized
}

fn power_iteration(matrix: &Vec<Vec<f64>>) -> Vec<f64> {
    let n = matrix.len();
    let mut vector = vec![1.0 / (n as f64).sqrt(); n];
    let mut previous_vector = vec![0.0; n];

    for _ in 0..MAX_ITERATIONS {
        // Multiply matrix with vector
        let mut new_vector = vec![0.0; n];
        for i in 0..n {
            for j in 0..n {
                new_vector[i] += matrix[i][j] * vector[j];
            }
        }

        // Normalize the resulting vector
        let magnitude: f64 = new_vector.iter().map(|&x| x*x).sum::<f64>().sqrt();
        for i in 0..n {
            new_vector[i] /= magnitude;
        }

        // Check convergence
        let max_difference: f64 = vector.iter().zip(&new_vector).map(|(&a, &b)| (a - b).abs()).fold(0.0, f64::max);
        if max_difference < THRESHOLD {
            return new_vector;
        }

        previous_vector = vector;
        vector = new_vector;
    }
    vector
}

fn eigen_trust(matrix: &Vec<Vec<f64>>) -> Vec<f64> {
    let normalized_matrix = normalize(matrix);
    power_iteration(&normalized_matrix)
}

fn main() {
    let matrix = vec![
        vec![1.0, 0.0, 1.0],
        vec![0.0, 1.0, 0.0],
        vec![0.0, 0.0, 1.0]
    ];
    let trust = eigen_trust(&matrix);
    println!("{:?}", trust);
}