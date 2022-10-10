use std::fs::File;
use std::io::{BufRead, BufReader, Read};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread;
use std::thread::JoinHandle;

use crossbeam_channel::{Receiver, Sender};
use indicatif::{ProgressBar, ProgressDrawTarget, ProgressStyle};
use zip::ZipArchive;

// finder
pub fn password_finder(
    zip_path: &str,
    password_list_path: &str,
    workers: usize,
    batch_size: usize,
) -> Option<String> {
    let zip_file_path = Path::new(zip_path);
    let password_list_file_path = Path::new(password_list_path).to_path_buf();

    let progress_bar = ProgressBar::new(0);
    let progress_style = ProgressStyle::default_bar()
        .template("[{elapsed_precise}] {wide_bar} {pos}/{len} throughput:{per_sec} (eta:{eta})")
        .expect("Failed to create progress style");
    progress_bar.set_style(progress_style);

    let draw_target = ProgressDrawTarget::stdout_with_hz(2);
    progress_bar.set_draw_target(draw_target);

    let file =
        BufReader::new(File::open(password_list_file_path.clone()).expect("Unable to open file"));
    let total_password_count = file.lines().count();
    progress_bar.set_length(total_password_count as u64);

    let (password_sender, password_receiver): (Sender<Vec<String>>, Receiver<Vec<String>>) =
        crossbeam_channel::bounded(workers * 10_000);

    let (found_password_sender, found_password_receiver): (Sender<String>, Receiver<String>) =
        crossbeam_channel::bounded(1);

    let stop_workers_signal = Arc::new(AtomicBool::new(false));
    let stop_gen_signal = Arc::new(AtomicBool::new(false));

    let password_gen_handle = start_password_reader(
        password_list_file_path,
        password_sender,
        batch_size,
        stop_gen_signal.clone(),
    );

    let mut worker_handles = Vec::with_capacity(workers);
    for i in 1..=workers {
        let join_handle = password_checker(
            i,
            zip_file_path,
            password_receiver.clone(),
            stop_workers_signal.clone(),
            found_password_sender.clone(),
            progress_bar.clone(),
        );
        worker_handles.push(join_handle);
    }

    drop(found_password_sender);

    match found_password_receiver.recv() {
        Ok(password_found) => {
            stop_gen_signal.store(true, Ordering::Relaxed);
            password_gen_handle.join().unwrap();
            stop_workers_signal.store(true, Ordering::Relaxed);
            for handle in worker_handles {
                handle.join().unwrap();
            }
            progress_bar.finish_and_clear();
            Some(password_found)
        }
        Err(_) => None,
    }
}

// reader
fn start_password_reader(
    file_path: PathBuf,
    password_sender: Sender<Vec<String>>,
    batch_size: usize,
    stop_signal: Arc<AtomicBool>,
) -> JoinHandle<()> {
    thread::Builder::new()
        .name("password-reader".to_string())
        .spawn(move || {
            let file = File::open(file_path).unwrap();
            let reader = BufReader::new(file);
            let mut batch = Vec::with_capacity(batch_size);
            for line in reader.lines() {
                batch.push(line.unwrap());
                if batch.len() == batch_size {
                    if stop_signal.load(Ordering::Relaxed) {
                        break;
                    } else {
                        match password_sender.send(batch.clone()) {
                            Ok(_) => batch.clear(),
                            Err(_) => break,
                        }
                    }
                }
            }
        })
        .unwrap()
}

// worker
fn password_checker(
    index: usize,
    file_path: &Path,
    password_receiver: Receiver<Vec<String>>,
    stop_signal: Arc<AtomicBool>,
    found_password_sender: Sender<String>,
    progress_bar: ProgressBar,
) -> JoinHandle<()> {
    let file = File::open(file_path).expect("File should exist");
    thread::Builder::new()
        .name(format!("worker-{}", index))
        .spawn(move || {
            let mut archive = ZipArchive::new(file).expect("Archive validated before-hand");
            while !stop_signal.load(Ordering::Relaxed) {
                match password_receiver.recv() {
                    Err(_) => break,
                    Ok(passwords) => {
                        let passwords_len = passwords.len() as u64;
                        for password in passwords {
                            let res = archive.by_index_decrypt(0, password.as_bytes());
                            match res {
                                Err(e) => panic!("Unexpected error {:?}", e),
                                Ok(Err(_)) => (), // invalid password
                                Ok(Ok(mut zip)) => {
                                    // ハッシュの衝突ではないことを検証する
                                    let mut buffer = Vec::with_capacity(zip.size() as usize);
                                    match zip.read_to_end(&mut buffer) {
                                        Err(_) => (), // password collision
                                        Ok(_) => {
                                            found_password_sender
                                                .send(password)
                                                .expect("Send found password should not fail");
                                        }
                                    }
                                }
                            }
                        }
                        progress_bar.inc(passwords_len);
                    }
                }
            }
        })
        .unwrap()
}
