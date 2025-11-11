use clap::Parser;
use std::collections::BTreeMap;
use std::fs;
use std::io::{self, Read};
use std::path::{Path, PathBuf};
use std::time::Instant;

#[derive(Parser)]
#[command(version, about = "Count word occurences in files")]
struct Args {
    path: PathBuf,
}

fn main() -> io::Result<()> {
    let args = Args::parse();
    let path = args.path;

    let start_time = Instant::now();

    let mut files = Vec::new();
    collect_files(&path, &mut files)?;

    let mut index: BTreeMap<String, BTreeMap<String, Vec<usize>>> = BTreeMap::new();
    for file in files {
        if let Err(e) = index_file(&file, &mut index) {
            println!("Failed to index {:?}: {}", file, e);
        }
    }

    let duration = start_time.elapsed();
    for (word, file_map) in &index {
        println!("{word}:");
        for (file, positions) in file_map {
            println!("  {file:?}: {:?}", positions);
        }
    }
    println!(
        "\nIndexed {} unique words in {:.3} seconds",
        index.len(),
        duration.as_secs_f64()
    );

    Ok(())
}

fn collect_files(path: &Path, files: &mut Vec<PathBuf>) -> io::Result<()> {
    if path.is_file() {
        files.push(path.to_path_buf());
    } else if path.is_dir() {
        for entry in fs::read_dir(path)? {
            let entry = entry?;
            let entry_path = entry.path();
            collect_files(&entry_path, files)?;
        }
    }
    Ok(())
}

fn index_file(
    path: &Path,
    index: &mut BTreeMap<String, BTreeMap<String, Vec<usize>>>,
) -> io::Result<()> {
    let mut file = fs::File::open(path)?;
    let mut data = String::new();
    match file.read_to_string(&mut data) {
        Ok(_) => {
            let words_positions = get_word_positions(&data);
            for (word, positions) in words_positions {
                let entry = index.entry(word.to_string()).or_insert_with(BTreeMap::new);
                let filepath = fs::canonicalize(path)?.to_string_lossy().to_string();
                entry
                    .entry(filepath)
                    .or_insert_with(Vec::new)
                    .extend(positions);
            }
        }
        Err(e) if e.kind() == io::ErrorKind::InvalidData => {
            return Err(io::Error::new(io::ErrorKind::InvalidData, "Invalid UTF-8"));
        }
        Err(e) => return Err(e),
    }

    Ok(())
}

fn get_word_positions(string_to_index: &str) -> BTreeMap<&str, Vec<usize>> {
    let mut indexed_string = BTreeMap::<&str, Vec<usize>>::new();

    let mut is_tracking_word = false;
    let mut word_start_index = 0;
    // Note on critical issue while using string_to_index.chars().enumerate().
    // "chars()" would iterate over the characters.
    // "enumerate()" will return the number of characters seen so far, NOT their byte positions.
    // Slicing a string requires byte! indices, because UTF-8 characters can span multiple bytes.
    // F.e, a word with 3 characters each spanning 2 bytes must be subscripted as [0..6].
    // Use of chars().enumerate() would result in [0..3], which is incorrect.
    for (index, ch) in string_to_index.char_indices() {
        let is_ch_punctuation = ch.is_ascii_punctuation() || ch.is_ascii_whitespace();

        if is_tracking_word {
            if is_ch_punctuation {
                let found_word = &string_to_index[word_start_index..index];
                indexed_string
                    .entry(found_word)
                    .or_default()
                    .push(word_start_index);
                is_tracking_word = false;
            }
        } else if !is_ch_punctuation {
            is_tracking_word = true;
            word_start_index = index;
        }
    }

    if is_tracking_word {
        let found_word = &string_to_index[word_start_index..];
        indexed_string
            .entry(found_word)
            .or_default()
            .push(word_start_index);
    }

    indexed_string
}
