extern crate pkg_config;

fn main() {


    println!("Build script started");

    if let Ok(lib) = pkg_config::probe_library("libuvc") {
        println!("Yes libuvc");
        for path in &lib.include_paths {
            println!("cargo:include={}", path.display());
        }
        return;
    } else {
        println!("No libuvc");
    }

}
