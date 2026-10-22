// The web app and the Remote Script are embedded into the binary.
fn main() {
    let dist = std::path::Path::new("../web/dist/index.html");
    if !dist.exists() {
        panic!("web/dist is missing: run `npm ci && npm run build` in web/ first");
    }
    println!("cargo:rerun-if-changed=../web/dist");
    println!("cargo:rerun-if-changed=../remote_script/LiveWeb");
}
