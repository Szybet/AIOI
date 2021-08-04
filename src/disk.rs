use core::num;
use log::info;
use log::{debug, error, log_enabled, Level};
use nix::libc::ioctl;
use nix::sys::statvfs::statvfs;
use pretty_env_logger::env_logger;
use regex::Regex;
use std::collections::HashMap;
use std::convert::TryInto;
use std::env;
use std::fs;
use std::fs::File;
use std::hash::Hash;
use std::io::prelude::*;
use std::io::BufReader;
use std::process;
use std::str;

macro_rules! cast {
    ($x:expr) => {
        u64::from($x)
    };
}

#[derive(Debug)]
pub struct Disksinfo {
    indexes: HashMap<String, Diskinfo>, // String is the linux name
}

#[derive(Debug)]
pub struct Diskinfo {
    modelname: Option<String>,
    hoursused: f32,
    size_in_mb: i32,
    used_in_mb: i32,
    free_in_mb: i32,
    partitions: Partitionsinfo,
}

#[derive(Debug)]
pub struct Partitionsinfo {
    indexes: HashMap<String, Partitioninfo>, // String is the name
}

#[derive(Debug)]
pub struct Partitioninfo {
    size_in_mb: f32,
    used_in_perc: f32,
    uuid: String,
    mountpoint: String,
    filesystemtype: String,
}

impl Disksinfo {
    pub fn disk(mut self) {
        let mut Partitionsinfostruct = Partitionsinfo {
            indexes: HashMap::new(),
        };
        let mut Diskinfostruct = Diskinfo {
            modelname: None,
            hoursused: 0.0,
            size_in_mb: 0,
            used_in_mb: 0,
            free_in_mb: 0,
            partitions: Partitionsinfostruct,
        };
        let paths = fs::read_dir("/sys/class/block/").unwrap();
        for path in paths {
            let mut path: String = path.unwrap().path().display().to_string();
            //info!("path: {:?}", path);
            let name = path.replace("/sys/class/block/", "");
            //info!("names: {:?}", name);

            let regex = Regex::new(r"[A-Za-z]").unwrap();
            let cuttednames = regex.replace_all(&name, "").to_string();
            //info!("Cutted: {:?}", cuttednames);
            if cuttednames == "" {
                //info!("disks: {:?}", &name);
                path.push_str(&String::from("/device/model"));
                info!("updated path: {:?}", &path);
                let file = File::open(&path).unwrap_or_else(|err| {
                    error!("not found {} error message: {}", &path, &err);
                    process::exit(1)
                });
                let file = BufReader::new(file);
                let mut file = file.lines();
                let line: String = file.nth(0).unwrap().unwrap();
                //info!("Model: {:?}", line);

                let mut Partitionsinfostruct = Partitionsinfo {
                    indexes: HashMap::new(),
                };

                let stat = statvfs("/".as_bytes()).unwrap(); // UNIX file blocksize
                let total_space: i32 =
                    ((cast!(stat.block_size()) * cast!(stat.blocks())) / 1024 / 1024)
                        .try_into()
                        .unwrap();
                let avail_space =
                    ((cast!(stat.block_size()) * cast!(stat.blocks_available())) / 1024 / 1024)
                        .try_into()
                        .unwrap();
                let used_space = total_space - avail_space;
                // the percentage is -5% than what shown by df due to
                // reserved blocks that we are currently not considering
                // https://github.com/giampaolo/psutil/issues/829
                info!("blocks {}", stat.blocks());
                info!("blocks size {}", stat.block_size());
                info!("total: {}", total_space);
                info!("avail: {}", avail_space);
                info!("used:  {}", used_space);

                Diskinfostruct = Diskinfo {
                    modelname: Some(line),
                    hoursused: 0.0,
                    size_in_mb: total_space,
                    used_in_mb: used_space,
                    free_in_mb: avail_space,
                    partitions: Partitionsinfostruct,
                };
                info!("Diskinfostruct {:?}", Diskinfostruct);
                self.indexes.insert(name, Diskinfostruct);
            }
        }

        println!("{:?}", &self);
    }

    pub fn new() -> Disksinfo {
        Disksinfo {
            indexes: HashMap::new(),
        }
    }
}
// size used
// https://docs.rs/nix/0.22.0/nix/#modules
// https://stackoverflow.com/questions/58624622/calculate-disk-usage-percentage-to-match-df-use-output

//df command, statvfs or statfs syscalls
//stat command if you want it formatted in a specific way

// https://mjmwired.net/kernel/Documentation/iostats.txt
// https://unix.stackexchange.com/questions/222735/how-to-get-hard-disk-information-from-proc-and-or-sys
// https://unix.stackexchange.com/questions/5085/how-to-see-disk-details-like-manufacturer-in-linux
