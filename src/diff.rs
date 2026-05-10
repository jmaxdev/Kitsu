use crate::storage::Storage;
use crate::objects::{Map, MapEntry};
use anyhow::Result;
use colored::*;
use similar::{ChangeTag, TextDiff};
use std::collections::{BTreeMap, BTreeSet};

pub fn diff_maps(storage: &Storage, old_map_hash: Option<&str>, new_map_hash: &str, path_prefix: &str) -> Result<()> {
    let old_entries = if let Some(hash) = old_map_hash {
        let (_, data) = storage.read_object(hash)?;
        Map::deserialize(&data)?.entries
    } else {
        Vec::new()
    };
    let (_, data) = storage.read_object(new_map_hash)?;
    let new_entries = Map::deserialize(&data)?.entries;
    let old_map: BTreeMap<String, MapEntry> = old_entries.into_iter().map(|e| (e.name.clone(), e)).collect();
    let new_map: BTreeMap<String, MapEntry> = new_entries.into_iter().map(|e| (e.name.clone(), e)).collect();
    let all_names: BTreeSet<String> = old_map.keys().cloned().chain(new_map.keys().cloned()).collect();
    for name in all_names {
        let old = old_map.get(&name);
        let new = new_map.get(&name);
        let full_path = if path_prefix.is_empty() { name.clone() } else { format!("{}/{}", path_prefix, name) };
        match (old, new) {
            (Some(o), Some(n)) => {
                if o.hash != n.hash {
                    if o.mode == "40000" && n.mode == "40000" {
                        diff_maps(storage, Some(&o.hash), &n.hash, &full_path)?;
                    } else {
                        println!("{} {}", "diff --vcontrol".bold(), full_path.bold());
                        let (_, old_data) = storage.read_object(&o.hash)?;
                        let (_, new_data) = storage.read_object(&n.hash)?;
                        print_diff(&old_data, &new_data);
                    }
                }
            }
            (Some(_o), None) => {
                println!("{} {}", "deleted file:".red().bold(), full_path.red());
            }
            (None, Some(n)) => {
                println!("{} {}", "new file:".green().bold(), full_path.green());
                let (_, new_data) = storage.read_object(&n.hash)?;
                print_diff(&[], &new_data);
            }
            (None, None) => unreachable!(),
        }
    }
    Ok(())
}

fn print_diff(old_data: &[u8], new_data: &[u8]) {
    let old_str = String::from_utf8_lossy(old_data);
    let new_str = String::from_utf8_lossy(new_data);
    let diff = TextDiff::from_lines(&old_str, &new_str);
    for change in diff.iter_all_changes() {
        let (sign, color) = match change.tag() {
            ChangeTag::Delete => ("-", Color::Red),
            ChangeTag::Insert => ("+", Color::Green),
            ChangeTag::Equal => (" ", Color::White),
        };
        print!("{}", format!("{}{}", sign, change).color(color));
    }
}
