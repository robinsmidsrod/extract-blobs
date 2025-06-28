use extract_blobs::Result;

fn main() -> Result<()> {
    let args = wild::args_os();
    extract_blobs::run(args)
}
