use anyhow::Result;
use kitsu_core::Repository;
use std::path::Path;

pub fn execute(current_dir: &Path, target: &str, output: &Path) -> Result<()> {
    let repo = Repository::open(current_dir)?;
    let hash = repo.resolve_target(target)?;
    let reachable = repo.collect_reachable(&hash)?;

    let file = std::fs::File::create(output)?;
    let enc = flate2::write::GzEncoder::new(file, flate2::Compression::default());
    let mut tar = tar::Builder::new(enc);

    let manifest = format!("target_name: {}\ntarget_hash: {}\n", target, hash);
    let mut manifest_header = tar::Header::new_gnu();
    manifest_header.set_size(manifest.len() as u64);
    manifest_header.set_mode(0o644);
    tar.append_data(&mut manifest_header, "MANIFEST", manifest.as_bytes())?;

    for h in reachable {
        let (obj_type, data) = repo.storage().read_object(&h)?;
        let mut header = tar::Header::new_gnu();
        header.set_size(data.len() as u64);
        header.set_mode(0o644);
        let path = format!("{}:{}", h, obj_type);
        tar.append_data(&mut header, path, &data[..])?;
    }
    tar.finish()?;
    println!("Exported {} to {:?}", target, output);
    Ok(())
}
