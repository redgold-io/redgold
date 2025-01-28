pub mod gembed;


use rand::Rng;
use std::collections::HashSet;
use std::fs::File;

struct Edge {
    src: u32,
    dst: u32,
    label: f64,
}

fn gen_test_data() -> Vec<Edge> {
    let mut rng = rand::thread_rng();
    let mut edges: Vec<Edge> = Vec::new();

    for vertex_id in 0..200 {
        let num_edges = rng.gen_range(5..=15);
        let mut added = HashSet::new();

        for _ in 0..num_edges {
            let other_vertex_id = rng.gen_range(0..200);
            if other_vertex_id != vertex_id && !added.contains(&other_vertex_id) {
                added.insert(other_vertex_id);
                let edge_value: f64 = rng.gen();
                edges.push(Edge {
                    src: vertex_id,
                    dst: other_vertex_id,
                    label: edge_value,
                });
            }
        }
    }
    edges
}

fn write_csv() {

    let edges = gen_test_data();

    let file = File::create("../../resources/graph_data.csv").expect("Could not create file");
    let mut wtr = csv::Writer::from_writer(file);

    for edge in edges {
        wtr.write_record(&[edge.src.to_string(), edge.dst.to_string(), edge.label.to_string()])
            .expect("Could not write record");
    }

    wtr.flush().expect("Could not flush writer");
}


#[ignore]
#[test]
fn debug() {
    write_csv();
}