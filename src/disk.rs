use log::error;
use log::info;
use nix::sys::statvfs::statvfs;
use regex::Regex;
use std::collections::HashMap;
use std::convert::TryInto;
use std::fs;
use std::fs::File;
use std::io::prelude::*;
use std::io::BufReader;
use std::path::Path;
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
    modelname: String,
    size_in_mb: i64,
    partitions: Partitionsinfo,
}

#[derive(Debug)]
pub struct Partitionsinfo {
    partitions: HashMap<String, Partitioninfo>, // String is the name
}

#[derive(Debug, Clone)]
pub struct Partitioninfo {
    size_in_mb: i64,
    used_in_mb: Option<i64>,
    free_in_mb: Option<i64>,
    uuid: String,
    mountpoint: Option<String>,
    filesystemtype: Option<String>,
}

impl Disksinfo {
    pub fn disk(mut self) {
        // Path to disks
        let mut paths_disks: Vec<_> = fs::read_dir("/sys/block/")
            .unwrap()
            .map(|r| r.unwrap())
            .collect();
        paths_disks.sort_by_key(|dir| dir.path());
        info!("Full path {:?}", &paths_disks);
        // For every disk
        for path_disk in paths_disks {
            let mut partitions_hash: HashMap<String, Partitioninfo> = HashMap::new();
            let path_disk: String = path_disk.path().display().to_string();
            info!("path disk: {:?}", path_disk);
            let name = path_disk.replace("/sys/block/", "");
            info!("name: {:?}", name);

            // Initializing the model name
            let mut model: String = String::new();
            let mut got_model: bool = false;

            // Size of the entire disk
            let path_size: String = path_disk.clone() + "/size";
            let file_size = File::open(&path_size).unwrap_or_else(|err| {
                error!("not found {} error message: {}", &path_size, &err);
                process::exit(1)
            });
            let file_size = BufReader::new(file_size);
            let mut file_size = file_size.lines();
            let mut size_disk_mb: i64 = file_size.nth(0).unwrap().unwrap().parse().unwrap();
            size_disk_mb = size_disk_mb * 512 / 1024 / 1024; // It is always 512, linux does that. it also counts anything to the size, like partition tables
            info!("size of disk: {}", size_disk_mb);

            // Getting partitions path
            let paths_partitions: Vec<_> = fs::read_dir(&path_disk).unwrap().collect();
            let mut names_partitions_only: Vec<String> = Vec::new();
            let mut regexexpression = path_disk.clone();
            regexexpression = regexexpression + &String::from(r"/\b");
            info!("regexexpression {:?}", regexexpression);
            // Get the names from partitions from /sys/block/*/*
            for path_partition in paths_partitions {
                let path_partition = path_partition.unwrap().path().display().to_string();
                info!("path partition {:?}", &path_partition);
                // Use regex to filter through pathes. if a path after cutting the disk path, contains the disk name its counted to the partition. for example sda1 contains sda
                let regex = Regex::new(&regexexpression).unwrap();
                let path_partition = regex.replace_all(&path_partition, "").to_string();
                info!("path partition after regex {:?}", path_partition);
                if path_partition.contains(&name) {
                    names_partitions_only.push(path_partition);
                }
            }
            names_partitions_only.sort();
            info!("names part only sort {:?}", &names_partitions_only);
            let mut part_info = Partitioninfo {
                size_in_mb: 0,
                used_in_mb: None,
                free_in_mb: None,
                uuid: String::new(),
                mountpoint: None,
                filesystemtype: None,
            };
            // Do opperations that are always available, like UUID and size
            for name_partition in &names_partitions_only {
                // get size
                info!("name partition in loop {:?}", name_partition);
                let mut path_part_size: String = path_disk.clone();
                path_part_size = path_part_size + "/";
                path_part_size = path_part_size + &name_partition;
                path_part_size = path_part_size + "/size";
                let file_part_size = File::open(path_part_size).unwrap();
                let file_part_size = BufReader::new(file_part_size);
                let mut file_part_size = file_part_size.lines();
                let file_part_size: i64 = file_part_size.nth(0).unwrap().unwrap().parse().unwrap();
                // sys path for udev
                let mut path_partition_sys: String = String::from("/sys/block/") + &name;
                path_partition_sys = path_partition_sys + "/";
                path_partition_sys = path_partition_sys + &name_partition;
                info!("path partition sys {:?}", &path_partition_sys);
                let path_partition_sys = Path::new(&path_partition_sys);
                let partition_device = udev::Device::from_syspath(path_partition_sys).unwrap();
                let mut uuid_partition: String = String::new();
                // get uuid
                let uuid_partition_option =
                    Disksinfo::udev_inf(&partition_device, String::from("ID_FS_UUID"));
                if uuid_partition_option == None {
                    uuid_partition = String::from("Error / does not exist");
                } else {
                    uuid_partition = uuid_partition_option.unwrap().to_str().unwrap().to_string();
                }
                info!("UUID {:?}", &uuid_partition);
                // Also get on time the model name of the disk from udev. its better than creating the device path 2 times
                if got_model == false {
                    got_model = true;
                    let mut model_option =
                        Disksinfo::udev_inf(&partition_device, String::from("ID_MODEL")); // /device/model sometimes dont work, udev is better
                    if model_option == None {
                        model_option =
                            Disksinfo::udev_inf(&partition_device, String::from("ID_NAME"));
                    }
                    if model_option == None {
                        model = String::from("Error");
                    } else {
                        model = model_option.unwrap().to_str().unwrap().to_string();
                    }
                }
                part_info = Partitioninfo {
                    size_in_mb: file_part_size,
                    used_in_mb: None,
                    free_in_mb: None,
                    uuid: uuid_partition,
                    mountpoint: None,
                    filesystemtype: None,
                };
                // insert the data to a the partition hashmap
                partitions_hash.insert(name_partition.clone(), part_info);
            }

            // Read /proc/mounts, and check if a file contains a partition name, if it contains then gather the used informations and modify the hashmap
            // The reason why those 2 loops above and below arent integrated is becouse to read the file one time, not many times for every partition
            let file_mounted = File::open("/proc/mounts").unwrap();
            let file_mounted = BufReader::new(file_mounted);
            let mut lines_mounted = file_mounted.lines();
            let mut part_fs: Option<&str>;
            let mut part_mount: Option<&str>;
            for line_mounted in &mut lines_mounted {
                let line_mounted: String = line_mounted.unwrap();
                for name_partition in &names_partitions_only {
                    if line_mounted.contains(name_partition) {
                        info!("line mounted: {:?}", &line_mounted);
                        info!("name partition: {:?}", &name_partition);
                        let mut line_mounted = line_mounted.split(" ");
                        part_mount = line_mounted.nth(1);
                        info!("part mount: {:?}", &part_mount);
                        part_fs = line_mounted.nth(0);
                        info!("part fs: {:?}", &part_fs);
                        let stat = statvfs(part_mount.unwrap().as_bytes()).unwrap(); // UNIX file blocksize
                        let total_space: i64 =
                            ((cast!(stat.block_size()) * cast!(stat.blocks())) / 1024 / 1024) // To mb
                                .try_into()
                                .unwrap();
                        let avail_space: i64 = ((cast!(stat.block_size())
                            * cast!(stat.blocks_available()))
                            / 1024
                            / 1024) // To mb
                            .try_into()
                            .unwrap();
                        let used_space = total_space - avail_space;
                        // the percentage is -5% than what shown by df due to
                        // reserved blocks that we are currently not considering
                        // https://github.com/giampaolo/psutil/issues/829
                        // https://docs.rs/nix/0.22.0/nix/#modules
                        // https://stackoverflow.com/questions/58624622/calculate-disk-usage-percentage-to-match-df-use-output
                        info!("available space: {}", avail_space);
                        info!("used space {}", used_space);

                        // Get a mutable reference to hashmap and edit it
                        let partition_hash_mup = partitions_hash
                            .get_mut(name_partition)
                            .expect("error get mut");
                        partition_hash_mup.used_in_mb = Some(used_space);
                        partition_hash_mup.free_in_mb = Some(avail_space);
                        partition_hash_mup.mountpoint = Some(part_mount.unwrap().to_string());
                        partition_hash_mup.filesystemtype = Some(part_fs.unwrap().to_string());
                        info!("partitions hash {:?}", partitions_hash);
                    }
                }
            }
            let part_info = Partitionsinfo {
                partitions: partitions_hash,
            };
            let diskstruct = Diskinfo {
                modelname: model,
                size_in_mb: size_disk_mb,
                partitions: part_info,
            };
            self.indexes.insert(name.clone(), diskstruct);
        }
        println!("{:?}", &self);
    }

    // udevadm info /dev/ "ID_FS_UUID"
    fn udev_inf(device: &udev::Device, property: String) -> std::option::Option<&std::ffi::OsStr> {
        device.property_value(property)
    }

    pub fn new() -> Disksinfo {
        Disksinfo {
            indexes: HashMap::new(),
        }
    }
}

/*
https://docs.rs/nix/0.22.0/nix/#modules
https://stackoverflow.com/questions/58624622/calculate-disk-usage-percentage-to-match-df-use-output
https://mjmwired.net/kernel/Documentation/iostats.txt
https://unix.stackexchange.com/questions/222735/how-to-get-hard-disk-information-from-proc-and-or-sys
https://unix.stackexchange.com/questions/5085/how-to-see-disk-details-like-manufacturer-in-linux
*/
