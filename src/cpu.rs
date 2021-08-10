use log::info;
use std::fs::File;
use std::fs::{self, DirEntry};
use std::io::prelude::*;
use std::io::BufReader;
use std::process;

#[derive(Debug)]
pub struct Cpuinfo {
    modelname: Option<String>,
    architecture: i32,
    cores: i32,
    threads: i32,
    maxmhz: i32,
    minmhz: i32,
    cpufreq: Vec<i32>,
    cache: Vec<Cache>,
}

#[derive(Debug)]
struct Cache {
    level: i32,
    instance: i32,
    size_in_k: i32,
    physical_memory: Option<String>,
}

impl Cpuinfo {
    pub fn cpu(mut self) {
        info!("Gathering cpu informations");
        let file = File::open("/proc/cpuinfo").unwrap();
        let file = BufReader::new(file);
        let mut count: i32 = 0;
        let mut lines = file.lines();
        // Get cpu informations from first lines, and then the cpu speed from other lines
        for line in &mut lines {
            let linestring: String = line.unwrap();
            count = Cpuinfo::basic_speed_info(&mut self, linestring, count);
        }

        // Get maxfreq and min freq from sys files
        let path_max: String =
            String::from("/sys/devices/system/cpu/cpu0/cpufreq/cpuinfo_max_freq");
        let path_min: String =
            String::from("/sys/devices/system/cpu/cpu0/cpufreq/cpuinfo_min_freq");
        Cpuinfo::freqlimit(path_max, &mut self.maxmhz);
        Cpuinfo::freqlimit(path_min, &mut self.minmhz);

        // Caches
        let mut paths: Vec<_> = fs::read_dir("/sys/devices/system/cpu/cpu0/cache/")
            .unwrap()
            .map(|r| r.unwrap())
            .collect();
        paths.sort_by_key(|dir| dir.path());
        for path in paths {
            Cpuinfo::cpu_cache(&mut self, path);
        }
        info!("{:?}", &self);
    }

    // Child Functions

    pub fn cpu_cache(&mut self, path: DirEntry) {
        // Get the needed informations
        let path: String = path.path().display().to_string();
        let indexname = path.replace("/sys/devices/system/cpu/cpu0/cache/", "");
        if indexname != "uevent" {
            let size = Self::readcachefile(String::from("/size"), indexname.clone());
            let level = Self::readcachefile(String::from("/level"), indexname.clone())
                .parse()
                .unwrap();
            // Make the output pretty
            let instanc: i32 = indexname.replace("index", "").parse().unwrap();
            let mut size_k: i32 = size.replace("K", "").parse().unwrap();
            let mut multiply = 0;
            let mut shacpulist_changed: Vec<String> = Vec::new();
            for corenumber in 0..self.threads {
                let mut pathtocores: String =
                    String::from("/sys/devices/system/cpu/cpu") + &corenumber.to_string();
                pathtocores = pathtocores + "/cache/index";
                pathtocores = pathtocores + &instanc.to_string();
                pathtocores = pathtocores + "/shared_cpu_list";
                let coresfile = File::open(&pathtocores).unwrap();
                let readedbuf = BufReader::new(coresfile);
                let mut readedbuf = readedbuf.lines();
                let readedlist: String = readedbuf.nth(0).unwrap().unwrap();
                if shacpulist_changed.contains(&readedlist) == false {
                    multiply = multiply + 1;
                    shacpulist_changed.push(readedlist)
                }
            }
            let mut psychical_mem_chip = size_k.to_string() + "x";
            size_k = size_k * multiply;
            psychical_mem_chip = psychical_mem_chip + &multiply.to_string();
            let exportstruct = Cache {
                level: level,
                instance: instanc,
                size_in_k: size_k,
                physical_memory: Some(psychical_mem_chip),
            };
            self.cache.push(exportstruct);
        }
    }

    pub fn basic_speed_info(&mut self, linestring: String, mut count: i32) -> i32 {
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
        count
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
        Cpuinfo {
            modelname: None,
            architecture: 0,
            cores: 0,
            threads: 0,
            maxmhz: 0,
            minmhz: 0,
            cpufreq: Vec::new(),
            cache: Vec::new(),
        }
    }
}
