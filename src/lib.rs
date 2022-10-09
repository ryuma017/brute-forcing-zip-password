use std::fs::File;
use std::io::{BufRead, BufReader, Read};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread;
use std::thread::JoinHandle;

use crossbeam_channel::{Receiver, Sender};

// reader
pub fn start_password_reader(
    file_path: PathBuf,
    send_password: Sender<String>,
    stop_signal: Arc<AtomicBool>,
) -> JoinHandle<()> {
    thread::Builder::new()
        .name("password-reader".to_string())
        .spawn(move || {
            let file = File::open(file_path).unwrap();
            let reader = BufReader::new(file);
            for line in reader.lines() {
                if stop_signal.load(Ordering::Relaxed) {
                    break;
                } else {
                    match send_password.send(line.unwrap()) {
                        Ok(_) => {}
                        Err(_) => break, // channel disconnected, stop thread
                    }
                }
            }
        })
        .unwrap()
}

// worker
pub fn password_checker(
    index: usize,
    file_path: &Path,
    receive_password: Receiver<String>,
    stop_signal: Arc<AtomicBool>,
    send_password_found: Sender<String>,
) -> JoinHandle<()> {
    let file = File::open(file_path).expect("File should exist");
    thread::Builder::new()
        .name(format!("worker-{}", index))
        .spawn(move || {
            let mut archive = zip::ZipArchive::new(file).expect("Archive validated before-hand");
            while !stop_signal.load(Ordering::Relaxed) {
                match receive_password.recv() {
                    Err(_) => break, // channel disconnected, stop thread
                    Ok(password) => {
                        let res = archive.by_index_decrypt(0, password.as_bytes()); // decrypt first file in archive
                        match res {
                            Err(e) => panic!("Unexpected error {:?}", e),
                            Ok(Err(_)) => (), // invalid password - continue
                            Ok(Ok(mut zip)) => {
                                // Validate password by reading the zip file to make sure it is not merely a hash collision.
                                let mut buffer = Vec::with_capacity(zip.size() as usize);
                                match zip.read_to_end(&mut buffer) {
                                    Err(_) => (), // password collision - continue
                                    Ok(_) => {
                                        // Send password and continue processing while waiting for signal
                                        send_password_found
                                            .send(password)
                                            .expect("Send found password should not fail");
                                    }
                                }
                            }
                        }
                    }
                }
            }
        })
        .unwrap()
}

// finder
pub fn password_finder(zip_path: &str, password_list_path: &str, workers: usize) -> Option<String> {
    let zip_file_path = Path::new(zip_path);
    let password_list_file_path = Path::new(password_list_path).to_path_buf();

    let (send_password, receive_password): (Sender<String>, Receiver<String>) =
        crossbeam_channel::bounded(workers * 10_000);

    let (send_found_password, receive_found_password): (Sender<String>, Receiver<String>) =
        crossbeam_channel::bounded(1);

    let stop_workers_signal = Arc::new(AtomicBool::new(false));
    let stop_gen_signal = Arc::new(AtomicBool::new(false));

    let password_gen_handle = start_password_reader(
        password_list_file_path,
        send_password,
        stop_gen_signal.clone(),
    );

    let mut worker_handles = Vec::with_capacity(workers);
    for i in 1..=workers {
        let join_handle = password_checker(
            i,
            zip_file_path,
            receive_password.clone(),
            stop_workers_signal.clone(),
            send_found_password.clone(),
        );
        worker_handles.push(join_handle);
    }

    drop(send_found_password);

    match receive_found_password.recv() {
        Ok(password_found) => {
            stop_gen_signal.store(true, Ordering::Relaxed);
            password_gen_handle.join().unwrap();
            stop_workers_signal.store(true, Ordering::Relaxed);
            for handle in worker_handles {
                handle.join().unwrap();
            }
            Some(password_found)
        }
        Err(_) => None,
    }
}
