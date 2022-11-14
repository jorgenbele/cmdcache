use std::{
    io::{self, Write},
    process::Command,
};

use clap::Parser;
use std::fs;
extern crate xdg;
use xdg::BaseDirectories;

use std::path::PathBuf;

use filetime::FileTime;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    // Cache command results for number of seconds
    #[arg(short, long, default_value_t = 60)]
    cache_seconds: u32,

    // Should we cache results from failed commands
    #[arg(long, default_value_t = false)]
    cache_failures: bool,

    // The command to execute
    command: String,
    command_args: Vec<String>,
}

fn get_cache_file_with_prefix(prefix: &str, name: &str, dirs: &BaseDirectories) -> PathBuf {
    let mut full_name = String::with_capacity(prefix.len() + name.len());
    full_name.push_str(prefix);
    full_name.push_str(name);
    return dirs.get_cache_file(full_name);
}

fn encode_command_args(command_args: &Vec<String>) -> String {
    let joined_args = command_args.join("\n");
    let encoded_args = base64::encode(joined_args);
    return encoded_args;
}

fn get_cached_paths(dirs: &BaseDirectories, encoded_args: &str) -> (PathBuf, PathBuf, PathBuf) {
    let exitcode_path = get_cache_file_with_prefix("exitcode_".into(), &encoded_args, dirs);
    let stdout_path = get_cache_file_with_prefix("stdout_".into(), &encoded_args, dirs);
    let stderr_path = get_cache_file_with_prefix("stderr_".into(), &encoded_args, dirs);
    return (exitcode_path, stdout_path, stderr_path);
}

fn get_cached_value<'a>(
    args: &Args,
    paths: &'a (PathBuf, PathBuf, PathBuf),
) -> Option<(i32, &'a PathBuf, &'a PathBuf)> {
    let meta = fs::metadata(&paths.0).ok()?;
    let ftime = FileTime::from_last_modification_time(&meta);
    let now_time = FileTime::now();

    let time_since_modification = now_time.seconds() - ftime.seconds();
    if time_since_modification >= args.cache_seconds.into() {
        // dbg!("Invalidated cache: is too old: {}", time_since_modification);
        return None;
    }

    let exitcode = match fs::read_to_string(&paths.0) {
        Ok(exitcode_str) => match exitcode_str.parse::<i32>() {
            Ok(exitcode) => exitcode,
            Err(_) => return None,
        },
        Err(_) => return None,
    };

    if exitcode != 0 && !args.cache_failures {
        // Command had non-zero exit code, and we don't cache that so ignore it
        return None;
    }

    return Some((exitcode, &paths.1, &paths.2));
}

fn run_and_put_cached_value<'a>(
    dirs: &BaseDirectories,
    args: &Args,
    paths: &'a (PathBuf, PathBuf, PathBuf),
) -> Option<(Option<i32>, &'a PathBuf, &'a PathBuf)> {
    let command_result = Command::new(args.command.clone())
        .args(args.command_args.clone())
        .output();

    let result = match command_result {
        Ok(result) => result,
        Err(_) => return None,
    };

    if dirs.place_cache_file(&paths.0).is_err() {
        return None;
    }
    if dirs.place_cache_file(&paths.1).is_err() {
        return None;
    }
    if dirs.place_cache_file(&paths.2).is_err() {
        return None;
    }

    if let Some(exitcode) = result.status.code() {
        if fs::write(&paths.0, exitcode.to_string()).is_err() {
            return None;
        };
    } else {
        return None;
    }

    if fs::write(&paths.1, &result.stdout).is_err() {
        return None;
    };
    if fs::write(&paths.2, &result.stderr).is_err() {
        return None;
    };

    return Some((result.status.code(), &paths.1, &paths.2));
}

fn display_cached_values(stdout: &PathBuf, stderr: &PathBuf) -> Result<(), io::Error> {
    let stdout_content = fs::read(stdout)?;
    io::stdout().write_all(stdout_content.as_slice())?;
    let stderr_content = fs::read(stderr)?;
    io::stderr().write_all(stderr_content.as_slice())?;
    return Ok(());
}

fn main() {
    let args = Args::parse();

    let dirs = xdg::BaseDirectories::with_prefix("cmdcache").expect("unable to get xdg dirs");
    dirs.create_cache_directory(args.command.clone())
        .expect("unable to create cache directories");
    // dbg!("cache_path: {:?}", path);

    let encoded_args = encode_command_args(&args.command_args);
    let paths = get_cached_paths(&dirs, &encoded_args);

    if let Some((exit_code, stdout, stderr)) = get_cached_value(&args, &paths) {
        // println!("Using cached value: {:?}", (exit_code, stdout, stderr));
        display_cached_values(stdout, stderr).expect("unable to read and write from cache");
        std::process::exit(exit_code);
    }

    if let Some((exit_code, stdout, stderr)) = run_and_put_cached_value(&dirs, &args, &paths) {
        display_cached_values(stdout, stderr).expect("unable to read and write from cache");
        std::process::exit(exit_code.unwrap_or(1));
    }
}
