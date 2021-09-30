#![feature(test)]

/// Program to generate winrates for Pokémon Showdown Random Battles
///
/// Written by Annika
/// Adapted from The Immortal's JavaScript winrate program, improved by Marty
extern crate test;
mod stats;
use rayon::prelude::*;
pub use stats::*;
use std::fs;
use std::path::PathBuf;
use std::sync::Mutex;
use structopt::StructOpt;

const PIKKR_TRAINING_ROUNDS: usize = 2;

#[derive(StructOpt)]
#[structopt(rename_all = "kebab-case")]
struct Options {
    #[structopt(long = "minimum-elo")]
    min_elo: u64,

    #[structopt(short = "i", long = "input")]
    #[structopt(parse(from_os_str))]
    format_dir: PathBuf,

    #[structopt(short = "o", long = "csv-output")]
    #[structopt(parse(from_os_str))]
    csv_output_path: Option<PathBuf>,

    #[structopt(short = "h", long = "human-output")]
    #[structopt(parse(from_os_str))]
    human_readable_output_path: Option<PathBuf>,

    #[structopt(long = "exclude")]
    exclusion: Option<String>,
}

fn handle_directory(
    min_elo: u64,
    format_dir: &PathBuf,
    exclusion: Option<String>,
) -> Result<stats::Stats, stats::StatsError> {
    let mut stats = Stats::new();
    let stats_mutex = Mutex::new(&mut stats);

    for entry in fs::read_dir(format_dir)? {
        let path = entry?.path();
        if path.is_dir() {
            let name = path.file_name().unwrap().to_str().unwrap_or("");
            let should_ignore = match exclusion {
                Some(ref x) => name.contains(x),
                None => false,
            };
            if should_ignore {
                println!("Ignoring {}", name);
                continue;
            }

            println!("Analyzing {}...", name);
            fs::read_dir(path)?
                .collect::<Vec<std::io::Result<fs::DirEntry>>>()
                .par_iter()
                .filter_map(|file| {
                    let mut json_parser = pikkr_annika::Pikkr::new(
                        &vec![
                            "$.p1rating.elo".as_bytes(), // p1 elo - idx 0
                            "$.p1team".as_bytes(),       // p1 team - idx 1
                            "$.p1".as_bytes(),           // p1 name - idx 2
                            "$.p2rating.elo".as_bytes(), // p2 elo - idx 3
                            "$.p2team".as_bytes(),       // p2 team - idx 4
                            "$.p2".as_bytes(),           // p2 name - idx 5
                            "$.winner".as_bytes(),       // winner - idx 6
                        ],
                        PIKKR_TRAINING_ROUNDS,
                    )
                    .unwrap();

                    let battle_json_path = file
                        .as_ref()
                        .expect(format!("error opening file").as_str())
                        .path();
                    let filename = battle_json_path.to_str().unwrap_or("");
                    if !filename.ends_with(".json") {
                        return None;
                    }

                    let data = fs::read_to_string(&battle_json_path)
                        .expect(format!("error reading file {}", filename).as_str());
                    let json = json_parser.parse(data.as_bytes()).unwrap();
                    Some(
                        Stats::process_json(min_elo, &json)
                            .expect(format!("error processing JSON in {}", filename).as_str()),
                    )
                })
                .for_each(|res| {
                    stats_mutex.lock().unwrap().add_game_results(res);
                });
        }
    }

    Ok(stats)
}

fn main() -> Result<(), StatsError> {
    let options = Options::from_args();

    if options.csv_output_path.is_none() && options.human_readable_output_path.is_none() {
        eprintln!("Error: You must specify at least one of --csv-output or --human-output");
        return Ok(());
    }

    let mut stats = handle_directory(options.min_elo, &options.format_dir, options.exclusion)?;

    if let Some(csv_path) = options.csv_output_path {
        fs::write(csv_path, stats.to_csv())?;
    }

    if let Some(human_path) = options.human_readable_output_path {
        fs::write(human_path, stats.to_human_readable())?;
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use lazy_static::lazy_static;
    use std::fs;
    use test::Bencher;

    lazy_static! {
        static ref TEST_DIR: PathBuf = PathBuf::from("target/test/day1");
    }

    fn build_test_dir(num_files: u32) -> std::io::Result<()> {
        let src_file = &PathBuf::from("src/benchmark-data.json");
        fs::create_dir_all(&TEST_DIR.clone())?;
        for i in 0..num_files {
            let mut file = TEST_DIR.clone();
            file.push(format!("{}.json", i));
            fs::copy(src_file, file)?;
        }
        Ok(())
    }

    #[bench]
    fn bench_handle_directory_1k(b: &mut Bencher) {
        build_test_dir(1_000).unwrap();

        let format_dir = &TEST_DIR.parent().unwrap().to_owned();
        b.iter(|| handle_directory(1050, format_dir, None).unwrap());
    }

    #[test]
    fn test_handle_directory_1k() {
        build_test_dir(1_000).unwrap();
        let format_dir = &TEST_DIR.parent().unwrap().to_owned();
        let mut stats = handle_directory(1050, format_dir, None).unwrap();

        assert_eq!(
            stats.to_csv(),
            "\"Rotom-Fan\",1000,1000,100,31.622776
\"Regirock\",1000,1000,100,31.622776
\"Conkeldurr\",1000,1000,100,31.622776
\"Reuniclus\",1000,1000,100,31.622776
\"Incineroar\",1000,1000,100,31.622776
\"Miltank\",1000,1000,100,31.622776
\"Drednaw\",1000,0,0,-31.622776
\"Pinsir\",1000,0,0,-31.622776
\"Pikachu\",1000,0,0,-31.622776
\"Latios\",1000,0,0,-31.622776
\"Entei\",1000,0,0,-31.622776
\"Exeggutor-Alola\",1000,0,0,-31.622776"
        );
        assert_eq!(
            stats.to_human_readable(),
            "+------+-----------------+------------+---------+-------+------+
| Rank | Pokemon         | Deviations | Winrate | Games | Wins |
+------+-----------------+------------+---------+-------+------+
| 1    | Rotom-Fan       | 31.622776  | 100%    | 1000  | 1000 |
+------+-----------------+------------+---------+-------+------+
| 2    | Regirock        | 31.622776  | 100%    | 1000  | 1000 |
+------+-----------------+------------+---------+-------+------+
| 3    | Conkeldurr      | 31.622776  | 100%    | 1000  | 1000 |
+------+-----------------+------------+---------+-------+------+
| 4    | Reuniclus       | 31.622776  | 100%    | 1000  | 1000 |
+------+-----------------+------------+---------+-------+------+
| 5    | Incineroar      | 31.622776  | 100%    | 1000  | 1000 |
+------+-----------------+------------+---------+-------+------+
| 6    | Miltank         | 31.622776  | 100%    | 1000  | 1000 |
+------+-----------------+------------+---------+-------+------+
| 7    | Drednaw         | -31.622776 | 0%      | 1000  | 0    |
+------+-----------------+------------+---------+-------+------+
| 8    | Pinsir          | -31.622776 | 0%      | 1000  | 0    |
+------+-----------------+------------+---------+-------+------+
| 9    | Pikachu         | -31.622776 | 0%      | 1000  | 0    |
+------+-----------------+------------+---------+-------+------+
| 10   | Latios          | -31.622776 | 0%      | 1000  | 0    |
+------+-----------------+------------+---------+-------+------+
| 11   | Entei           | -31.622776 | 0%      | 1000  | 0    |
+------+-----------------+------------+---------+-------+------+
| 12   | Exeggutor-Alola | -31.622776 | 0%      | 1000  | 0    |
+------+-----------------+------------+---------+-------+------+
"
        )
    }
}
