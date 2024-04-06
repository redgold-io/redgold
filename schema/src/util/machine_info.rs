use itertools::Itertools;
use crate::{error_info, ErrorInfoContext, RgResult, SafeOption};
use crate::observability::errors::EnhanceErrorInfo;
use crate::util::cmd;
use crate::util::cmd::run_bash;

// This should really be in a separate module

pub fn file_size_bytes(path: impl Into<String>) -> RgResult<i64> {
    let out = run_bash(format!("wc -c < {}", path.into()))?.0;
    let size = out.split_whitespace().next().ok_msg("Missing size from du command")?
        .parse::<i64>().error_info("Failed to parse size")?;
    Ok(size)
}

pub fn available_bytes(path: impl Into<String>, is_windows: bool) -> RgResult<i64> {
    if is_windows {
        return Err(error_info("available_bytes Not implemented for Windows"));
    }

    let out = cmd::run_bash(format!("df {}", path.into()))?.0;

    /*
    Mac:
    Filesystem   512-blocks       Used Available Capacity iused      ifree %iused  Mounted on
    /dev/disk3s5 1942700360 1341143528 552360152    71% 4990532 2761800760    0%   /System/Volumes/Data
    Linux:
    Filesystem     1K-blocks      Used Available Use% Mounted on
    /dev/sda3      456042808 160028204 272775436  37% /
     */
    let mut split = out.split("\n");
    let head = split.next().ok_msg("Missing stdout from df disk space command")?;

    // Use regex to split lines by one or more whitespace characters
    // let re = Regex::new(r"\s+").expect("Failed to compile regex");

    let string = head.split_whitespace().dropping(1).next().ok_msg("Missing second column from df disk space command")?
        .split("-").next().ok_msg("Missing first part of second column from df disk space command")?.to_lowercase();
    let blocks = string.as_str();

    let multiplier: i64 = match blocks {
        "1k" => {1024}
        "512" => {512}
        _ => {
            return Err(error_info(format!("Unknown block size: {}", blocks)))
        }
    };
    let second = split.next().ok_msg("Missing second line from df disk space command")?;
    let available_bytes = second.split_whitespace().dropping(3).next()
        .ok_msg("Missing fourth column from df disk space command")?
        .parse::<i64>().error_info("Failed to parse available bytes")? * multiplier;
    Ok(available_bytes)

}

#[test]
fn test_disk() {
    let b = available_bytes("~", false).unwrap();
    let gb = b / (1024 * 1024 * 1024);
    println!("Available bytes in gb: {}", gb);
}

const GET_DISK_SPACE_CMD: &str = r#"df -Ph . | awk 'NR==2 {print $4}'"#;
pub fn get_disk_space() -> RgResult<u32> {
    let (stdout, stderr) = run_bash(GET_DISK_SPACE_CMD)?;
    let gigs = stdout.split("G").next();
    let gb = gigs.safe_get_msg("Err in getting disk space").add(stderr.clone())?;
    gb.parse::<u32>().error_info("Err in parsing disk space").add(stderr.clone())
}

#[test]
pub fn disk_space_test() {
    let gb = get_disk_space().expect("");
    println!("Gigs disk {gb}");
    let b = available_bytes("~", false).unwrap();
    let gb = b / (1024 * 1024 * 1024);
    println!("Available bytes in gb: {}", gb);
}


pub fn memory_total_kb() -> RgResult<i64> {
    let cmd = "grep MemTotal /proc/meminfo";
    let (out, err) = run_bash(cmd)?;
    let total = out.split_whitespace().nth(1).ok_msg("Missing total memory from /proc/meminfo")?
        .parse::<i64>().error_info("Failed to parse total memory")
        .add(err)?;
    Ok(total)
}
pub fn cores_total() -> RgResult<i64> {
    let cmd = "nproc";
    let (out, err) = run_bash(cmd)?;
    let total = out.trim().parse::<i64>().error_info("Failed to parse total cores with nproc")
        .add(err)?;
    Ok(total)
}