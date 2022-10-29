
// TODO: Per network environment, deal with lb for now.

pub struct Seed {
    pub host: String
}

impl Seed {
    pub fn new(host: String) -> Self {
        Self { host }
    }
}

fn get_seeds() {
    vec![
        Seed::new("hostnoc.redgold.io")
    ]
}