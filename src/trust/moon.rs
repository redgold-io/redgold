use crate::util::current_time_unix;
use chrono::{DateTime, Local, NaiveDateTime};

// https://gist.github.com/mkiang/7841b1c8ddb6feecb2e9d2fbbd93f839
pub fn load_moons() -> Vec<u64> {
    let str = include_str!("../resources/full_moon.csv");
    let lines = str.split("\n");
    let mut i = 0;
    let mut timestamps: Vec<u64> = vec![];
    for line in lines {
        i += 1;
        if i == 1 {
            continue;
        }
        let split = line.split(", ").collect::<Vec<&str>>();

        let time = split
            .get(2)
            .unwrap()
            .replace("[**]", "")
            .replace("[+]", "")
            .replace("[*]", "");
        let date_str = format!("{} {} +0200", split.get(1).unwrap(), time);
        // println!("Date_str: {}", date_str);

        let dt = DateTime::parse_from_str(
            &*date_str,
            //Wednesday, 28 December 2050, 06:15:36 am
            "%d %B %Y %I:%M:%S %P %z", // "1983 Apr 13 12:09:14.274 +0000",
                                       // "%Y %b %d %H:%M:%S%.3f %z"
        )
        .unwrap();

        let local = Local::today().timezone();
        let dt2 = dt.with_timezone(&local);
        let timestamp = dt2.timestamp();
        if timestamp > 0 {
            timestamps.push(timestamp as u64);
        }
        // println!("{}", dt2.naive_local().to_string());
    }
    //
    // for ts in timestamps {
    //     let local = Local::today().offset().clone();
    //     let dt = DateTime::<Local>::from_utc(NaiveDateTime::from_timestamp(ts, 0), local);
    //     println!("Dt {}", dt.to_string());
    // }
    return timestamps;
}

fn unix_to_local_date(unix: u64) -> DateTime<Local> {
    let local = Local::today().offset().clone();
    let dt = DateTime::<Local>::from_utc(NaiveDateTime::from_timestamp(unix as i64, 0), local);
    return dt;
}

pub fn next_reward_time(vec: Vec<u64>) -> u64 {
    let cur = current_time_unix();
    // for v in vec.clone() {
    //     println!("{:?}", v);
    // }
    // println!("{:?}", cur.clone());
    let next = vec.iter().filter(|t| **t > cur).min().unwrap().clone();
    next
}

#[test]
fn test() {
    let vec = load_moons();
    let r = next_reward_time(vec);
    let dt = unix_to_local_date(r);
    println!("Dt {}", dt.to_string());
}
