use std::collections::HashMap;

pub struct Resources {
    host_manager_service_script: String,
    full_moon_csv: String,
    pub logo_bytes: Vec<u8>,
    filebeat_docker_config: String,
    btc_rpc_auth_py: String,
    // Doesn't appear to be possible to generate automatically?
    file_lookup: HashMap<String, Vec<u8>>,
    pub redgold_docker_compose: String,
}

// Change this to list all files? For table scripts?
impl Default for Resources {
    fn default() -> Self {
        Resources {
            host_manager_service_script: include_str!(
                "infra/service_scripts/redgold-host-manager.service"
            )
            .to_string(),
            full_moon_csv: include_str!("full_moon.csv").into(),
            logo_bytes: include_bytes!("images/historical/design_one/logo_orig_crop.png").to_vec(),
            // logo_bytes: include_bytes!("logo.jpg").to_vec(),
            filebeat_docker_config: include_str!("infra/ops_services/filebeat.docker.yml").into(),
            btc_rpc_auth_py: include_str!("infra/experimental/rpcauth.py").into(),
            file_lookup: HashMap::new(),
            redgold_docker_compose: include_str!("infra/redgold-only.yml").into(),
        }
    }
}
/*
fn process(from: &Path, to: &Path) -> IoResult<()> {
    // creates a new tempdir with the specified suffix
    let tempdir = try!(TempDir::new("skylight"));

    // open the input file
    let mut from_file = try!(File::open(from));

    // create a temporary file inside the tempdir
    let mut tempfile =
        try!(File::create(&tempdir.path().join("tmp1")));

    // copy the input file into the tempfile
    try!(io::util::copy(&mut from_file, &mut tempfile));

    // use an external program to process the tmpfile in place

    // after processing, copy the tempfile into the output file
    let mut out = try!(File::create(to));

    io::util::copy(&mut tempfile, &mut out)
 */
impl Resources {}
