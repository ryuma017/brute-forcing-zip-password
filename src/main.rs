use std::env;

use zip_pw_finder::password_finder;

fn main() {
    let mut args_iter = env::args().skip(1);
    let zip_path = args_iter
        .next()
        .expect("Path to the ZIP file must be specified");
    let dictionary_path = "passwords/xato-net-10-million-passwords.txt";

    let batch_size = args_iter.next().unwrap().parse::<usize>().unwrap();
    let workers = num_cpus::get_physical();

    match password_finder(&zip_path, dictionary_path, workers, batch_size) {
        Some(password_found) => println!("Password found: {password_found}"),
        None => println!("No password found :("),
    }
}
