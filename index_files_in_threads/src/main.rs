use clap::Parser;
use std::collections::BTreeMap;
use std::fs;
use std::io::{self, Read};
use std::path::{Path, PathBuf};
use std::sync::mpsc::{self, Receiver};
use std::sync::{Arc, Mutex};
use std::thread::JoinHandle;
use std::time::Instant;

#[derive(Parser)]
#[command(version, about = "Count word occurences in files")]
struct Args {
    path: PathBuf,
}

#[derive(Debug)]
struct IndexResults {
    filename: PathBuf,
    index: BTreeMap<String, Vec<usize>>,
}

struct ThreadedFileIndexer {
    worker_threads: Vec<JoinHandle<()>>,
    rx: Receiver<IndexResults>, // this Receiver will be used by user of the threaded file indexer
                                // to retrieve results of processing
                                // tx: Sender<BTreeMap<String, Vec<usize>>>, // Sender will be cloned to each of the worker threads and will be used to send results
}

impl ThreadedFileIndexer {
    fn new(files_to_process: Vec<PathBuf>) -> ThreadedFileIndexer {
        let threads_num = num_cpus::get() - 1; // account for the main thread which is already running
        eprintln!("Will spawn {} additional threads", threads_num);

        let shared_job_queue = Arc::new(Mutex::new(files_to_process));
        let (tx, rx) = mpsc::channel();
        // this tx will be dropped as we are done with initializing ThreadedFileIndexer object

        let mut threads = Vec::new();
        for i in 0..threads_num {
            let job_queue = shared_job_queue.clone();
            let tx = tx.clone();
            threads.push(std::thread::spawn(move || {
                let thread_id = i;
                eprintln!("Spawned thread {thread_id}");
                loop {
                    let Some(filename) = job_queue.lock().unwrap().pop() else {
                        break; // shut down this thread if "job_queue" is empty
                    };

                    match index_file(&filename) {
                        Ok(word_pos) => {
                            tx.send(IndexResults {
                                filename,
                                index: word_pos,
                            })
                            .unwrap_or_else(|e| {
                                println!("Failed to send results from {thread_id}: {e}")
                            });
                        }
                        Err(e) if e.kind() == io::ErrorKind::InvalidData => {
                            println!("Skip {:?} - Invalid UTF8", filename)
                        }
                        Err(e) => {
                            println!("Skip {:?} - {e}", filename)
                        }
                    }
                }
                // at this point our tx end will be dropped
                // as all threads shut down their tx ends, main thread will get its rx end closed automatically (it will receive notification)
            }));
        }

        return ThreadedFileIndexer {
            worker_threads: threads,
            rx,
        };
    }

    fn get_rx(&self) -> &mpsc::Receiver<IndexResults> {
        &self.rx
    }

    fn join_all(self) {
        for thread in self.worker_threads {
            thread
                .join()
                .unwrap_or_else(|e| println!("Failed to join thread: {:?}", e));
        }
    }
}

fn main() -> io::Result<()> {
    let args = Args::parse();
    let path = args.path;

    let start_time = Instant::now();

    let mut files = Vec::new();
    collect_files(&path, &mut files)?;

    let threaded_file_indexer = ThreadedFileIndexer::new(files);

    let mut index: BTreeMap<String, BTreeMap<String, Vec<usize>>> = BTreeMap::new();
    let rx_end = threaded_file_indexer.get_rx();
    for result in rx_end.iter() {
        let filename = result.filename;
        let word_pos = result.index;
        println!("Received {:?}", filename);

        for (word, positions) in word_pos {
            let entry = index.entry(word.to_string()).or_insert_with(BTreeMap::new);

            let filepath = fs::canonicalize(filename.to_path_buf())?
                .to_string_lossy()
                .to_string();

            entry
                .entry(filepath)
                .or_insert_with(Vec::new)
                .extend(positions);
        }
    }

    let duration = start_time.elapsed();
    println!("Indexing results: {:?}", index);
    println!("Done indexing files in {} seconds", duration.as_secs_f64());

    threaded_file_indexer.join_all();

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

fn index_file(path: &Path) -> io::Result<BTreeMap<String, Vec<usize>>> {
    let mut file = fs::File::open(path)?;
    let mut data = String::new();

    match file.read_to_string(&mut data) {
        Ok(_) => return Ok(get_word_positions(&data)),
        Err(e) => return Err(e),
    }
}

fn get_word_positions(string_to_index: &str) -> BTreeMap<String, Vec<usize>> {
    let mut indexed_string = BTreeMap::<String, Vec<usize>>::new();

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
                    .entry(found_word.to_string())
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
            .entry(found_word.to_string())
            .or_default()
            .push(word_start_index);
    }

    indexed_string
}
