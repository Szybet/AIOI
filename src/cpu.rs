use std::collections::HashMap;
use std::fs;
use std::fs::File;
use std::hash::Hash;
use std::io::prelude::*;
use std::io::BufReader;
use std::process;
use std::str;

#[derive(Debug)]
pub struct Cpuinfo {
    modelname: Option<String>,
    architecture: i32,
    cores: i32,
    threads: i32,
    maxmhz: i32,
    minmhz: i32,
    cpufreq: Vec<i32>,
    cash: Vec<Cache_inf_hr>,
}

#[derive(Debug, Clone)]
pub struct Cacheinformation {
    level: i32,
    size: Option<String>,
    shared_beginning: i32,
    shared_end: i32,
    shared_type: Option<String>,
}

#[derive(Debug)]
pub struct Cache_inf_hr {
    level: i32,
    instance: i32,
    size_in_K: i32,
    physical_memory: Option<String>,
}

#[derive(Debug)]
pub struct Cpucaches {
    indexes: HashMap<String, Cacheinformation>,
}

impl Cpuinfo {
    pub fn cpu(mut self) {
        println!("Gathering cpu informations");

        let file = File::open("/proc/cpuinfo").unwrap();
        let file = BufReader::new(file);

        let mut count: i32 = 0;
        let mut lines = file.lines();
        for line in &mut lines {
            let linestring: String = line.unwrap();
            // Get cpu informations from first lines
            if count < 28 {
                count = count + 1;

                if linestring.contains("model name") {
                    let linestring = linestring.replace("model name\t: ", "");
                    self.modelname = Some(linestring);
                }
                if linestring.contains("model name") {
                    let linestring = linestring.replace("model name\t: ", "");
                    self.modelname = Some(linestring);
                }
                if linestring.contains("flags\t\t:") {
                    if linestring.contains("lm") {
                        self.architecture = 64;
                    } else if linestring.contains("pm") {
                        self.architecture = 32;
                    } else if linestring.contains("rl") {
                        self.architecture = 16;
                    }
                }
                if linestring.contains("cpu cores\t:") {
                    let linestring = linestring.replace("cpu cores\t: ", "");
                    let corecount: i32 = linestring.parse().unwrap();
                    self.cores = corecount;
                }
                if linestring.contains("siblings\t:") {
                    let linestring = linestring.replace("siblings\t: ", "");
                    let threadscount: i32 = linestring.parse().unwrap();
                    self.threads = threadscount;
                }
            }
            // Get cpu speed per core
            if linestring.contains("cpu MHz") {
                let operline = linestring.replace("cpu MHz\t\t: ", "");
                let operline = operline.split(".").nth(0).unwrap();
                //mhzvector.push(operline.parse().unwrap());
                self.cpufreq.push(operline.parse().unwrap());
            }
        }

        // Get maxfreq and min freq from sys files
        let pathmax: String = String::from("/sys/devices/system/cpu/cpu0/cpufreq/cpuinfo_max_freq");
        let pathmin: String = String::from("/sys/devices/system/cpu/cpu0/cpufreq/cpuinfo_min_freq");
        Cpuinfo::freqlimit(pathmax, &mut self.maxmhz);
        Cpuinfo::freqlimit(pathmin, &mut self.minmhz);

        // Caches

        let paths = fs::read_dir("/sys/devices/system/cpu/cpu0/cache/").unwrap();

        let mut cpucachedata = Cpucaches {
            indexes: HashMap::new(),
        };
        let mut hashinsert = HashMap::new();

        for path in paths {
            let path: String = path.unwrap().path().display().to_string();
            let indexname = path.replace("/sys/devices/system/cpu/cpu0/cache/", "");
            let mut gatheredinformations = Cacheinformation {
                level: 0,
                size: None,
                shared_beginning: 0,
                shared_end: 0,
                shared_type: None,
            };
            if indexname != "uevent" {
                let size = Self::readcachefile(String::from("/size"), indexname.clone());
                let level = Self::readcachefile(String::from("/level"), indexname.clone())
                    .parse()
                    .unwrap();
                let shared_cpu_list =
                    Self::readcachefile(String::from("/shared_cpu_list"), indexname.clone());
                let mut split_begin: i32 = 0;
                let mut split_end: i32 = 0;
                let mut split_type: String = String::new();
                if shared_cpu_list.contains(",") {
                    let mut splitting = shared_cpu_list.split(",");
                    split_begin = splitting.nth(0).unwrap().parse().unwrap();
                    split_end = splitting.nth(0).unwrap().parse().unwrap();
                    split_type = String::from(",");
                }
                if shared_cpu_list.contains("-") {
                    let mut splitting = shared_cpu_list.split("-");
                    split_begin = splitting.nth(0).unwrap().parse().unwrap();
                    split_end = splitting.nth(0).unwrap().parse().unwrap();
                    split_type = String::from("-");
                }
                gatheredinformations = Cacheinformation {
                    level: level,
                    size: Some(size),
                    shared_beginning: split_begin,
                    shared_end: split_end,
                    shared_type: Some(split_type),
                };
                hashinsert.insert(indexname, gatheredinformations);
            }
        }
        cpucachedata = Cpucaches {
            indexes: hashinsert,
        };

        for (stringindex, structindex) in cpucachedata.indexes {
            let instanc: i32 = stringindex.replace("index", "").parse().unwrap();
            let sizeparsed = structindex.size.unwrap().replace("K", "").parse().unwrap();
            let mut size: i32 = sizeparsed;
            let mut psychical_mem_chip: String = sizeparsed.to_string() + "x";
            if structindex.shared_type.unwrap().contains(",") {
                size = size * structindex.shared_end.clone();
                psychical_mem_chip = psychical_mem_chip + &structindex.shared_end.to_string();
            } else {
                psychical_mem_chip = psychical_mem_chip + "1";
            }
            let exportstruct = Cache_inf_hr {
                level: structindex.level,
                instance: instanc,
                size_in_K: size,
                physical_memory: Some(psychical_mem_chip),
            };
            self.cash.push(exportstruct);
        }
        // https://en.wikichip.org/wiki/intel/core_i5/i5-8300h
        // https://www.kernel.org/doc/Documentation/ABI/testing/sysfs-devices-system-cpu
        // https://www.google.com/search?q=8192%20KB%20to%20mb

        //https://superuser.com/questions/405355/meaning-of-files-in-cpu-folder-of-linux
        // https://unix.stackexchange.com/questions/113555/when-speaking-about-cache-size-of-a-cpu-we-only-need-the-size-of-the-cache-at-t
        println!("{:?}", &self);
    }

    pub fn readcachefile(indexfile: String, indexname: String) -> String {
        let mut indexpath: String = String::from("/sys/devices/system/cpu/cpu0/cache/");
        indexpath.push_str(&indexname);
        indexpath.push_str(&indexfile);
        let file = File::open(&indexpath).unwrap_or_else(|err| {
            println!("not found {} error message: {}", &indexpath, &err);
            process::exit(1)
        });
        let file = BufReader::new(file);
        let mut file = file.lines();
        let size: String = file.nth(0).unwrap().unwrap();
        size
    }

    pub fn freqlimit(path: String, value: &mut i32) {
        let file = File::open(&path).unwrap_or_else(|err| {
            println!("not found {} error message: {}", &path, &err);
            process::exit(1)
        });
        let file = BufReader::new(file);
        let mut file = file.lines();
        let maxfreq: i32 = file.nth(0).unwrap().unwrap().parse().unwrap();
        let maxfreq = maxfreq / 1000;
        *value = maxfreq;
    }

    pub fn new() -> Cpuinfo {
        /*
        Cacheinformation {
            level: 0,
            size: 0,
            shared_beginning: 0,
            shared_end: 0,
            shared_type: None,
        };
        */

        let index = HashMap::new();
        Cpucaches { indexes: index };

        Cpuinfo {
            modelname: None,
            architecture: 0,
            cores: 0,
            threads: 0,
            maxmhz: 0,
            minmhz: 0,
            cpufreq: Vec::new(),
            cash: Vec::new(),
        }
    }
}
