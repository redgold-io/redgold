use prost_serde::BuildConfig;

fn main() {
    println!("Build script started");

    let json = include_str!("json_build_config.json");
    //prost_build::compile_protos(&["src/structs.proto"], &["src"]).unwrap();
    // prost_serde::build_with_serde(json);
    let build_config: BuildConfig = serde_json::from_str(json).unwrap();

    let mut config = prost_build::Config::new();
    for opt in build_config.opts.iter() {
        match opt.scope.as_ref() {
            "bytes" => {
                config.bytes(&opt.paths);
                continue;
            }
            "btree_map" => {
                config.btree_map(&opt.paths);
                continue;
            }
            _ => (),
        };
        for path in opt.paths.iter() {
            match opt.scope.as_str() {
                "type" => config.type_attribute(path, opt.attr.as_str()),
                "field" => config.field_attribute(path, opt.attr.as_str()),
                v => panic!("Not supported type: {}", v),
            };
        }
    }

    config.extern_path(".serde_as", "serde_with::serde_as");

    // fs::create_dir_all(&build_config.output).unwrap();
    // config.out_dir(&build_config.output);

    config
        .compile_protos(&build_config.files, &build_config.includes)
        .unwrap_or_else(|e| panic!("Failed to compile proto files. Error: {:?}", e));
    //
    // Command::new("cargo")
    //     .args(&["fmt"])
    //     .status()
    //     .expect("cargo fmt failed");

    //build_config
}
