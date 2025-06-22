fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = wild::args_os();
    extract_blobs::run(args)
}
