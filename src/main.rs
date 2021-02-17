/// Program to generate winrates for Pokémon Showdown Random Battles
///
/// Written by Annika
/// Adapted from The Immortal's JavaScript winrate program, improved by Marty

mod stats;
use stats::*;
use std::fs;
use std::path::PathBuf;
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
}

fn main() -> Result<(), StatsError> {
    let options = Options::from_args();

    if options.csv_output_path.is_none() && options.human_readable_output_path.is_none() {
        eprintln!("Error: You must specify at least one of --csv-output or --human-output");
        return Ok(());
    }

    let mut stats = Stats::new(options.min_elo);

    let mut json_parser = pikkr_annika::Pikkr::new(&vec![
        "$.p1rating.elo".as_bytes(), // p1 elo - idx 0
        "$.p1team".as_bytes(), // p1 team - idx 1
        "$.p1".as_bytes(), // p1 name - idx 2

        "$.p2rating.elo".as_bytes(), // p2 elo - idx 3
        "$.p2team".as_bytes(), // p2 team - idx 4
        "$.p2".as_bytes(), // p2 name - idx 5

        "$.winner".as_bytes(), // winner - idx 6
    ], PIKKR_TRAINING_ROUNDS)?;

    for entry in fs::read_dir(options.format_dir)? {
        let path = entry?.path();
        if path.is_dir() {
            println!("Analyzing {}...", path.file_name().unwrap().to_str().unwrap_or(""));
            for file in fs::read_dir(path)? {
                let battle_json_path = file?.path();
                let filename = battle_json_path.to_str().unwrap_or("");
                if !filename.ends_with(".json") {
                    continue;
                }

                let data = fs::read_to_string(battle_json_path)?;
                let json = json_parser.parse(data.as_bytes()).unwrap();
                stats.process_json(json)?;
            }
        }
    }


    if let Some(csv_path) = options.csv_output_path {
        fs::write(csv_path, stats.to_csv())?;
    }

    if let Some(human_path) = options.human_readable_output_path {
        fs::write(human_path, stats.to_human_readable())?;
    }

    Ok(())
}
