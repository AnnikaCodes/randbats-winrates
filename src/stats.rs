/// Stats code
extern crate test;
use indexmap::IndexMap;
use itertools::Itertools;
use prettytable::*;

pub trait Output {
    fn to_human_readable(&mut self) -> String;
    fn to_csv(&mut self) -> String;
}

#[derive(Copy, Clone)]
struct FinalStats {
    /// as percentage
    winrate: f32,
    deviations: f32,
}

#[derive(Debug)]
pub enum StatsError {
    Team(String),
    IO(std::io::Error),
    JSON(pikkr_annika::Error),
}

impl From<String> for StatsError {
    fn from(str: String) -> StatsError {
        StatsError::Team(str)
    }
}

impl From<std::io::Error> for StatsError {
    fn from(err: std::io::Error) -> StatsError {
        StatsError::IO(err)
    }
}

impl From<pikkr_annika::Error> for StatsError {
    fn from(err: pikkr_annika::Error) -> StatsError {
        StatsError::JSON(err)
    }
}

/// Stores statistics about a pokemon
#[derive(Copy, Clone, Debug)]
struct PokemonStats {
    games: u32,
    wins: u32,
}

impl PokemonStats {
    /// Computes the number of standard deviations from the average
    fn final_stats(&self) -> FinalStats {
        let games = self.games as f32;
        let winrate = (self.wins as f32 / games) * 100.0;

        // Standard deviations formula courtesy of pyuk (@pyuk-bot on GitHub)
        let deviations = (winrate - 50.0) * games.sqrt() / 50.0;

        FinalStats {
            winrate,
            deviations,
        }
    }
}

#[derive(Debug)]
pub struct GameResult {
    species: String,
    won: bool,
}

/// Stores overall statistics
#[derive(Debug)]
pub struct Stats {
    /// Pokemon:statistics map
    pokemon: IndexMap<String, PokemonStats>,
    is_sorted: bool,
}

impl Stats {
    pub fn new() -> Self {
        Self {
            pokemon: IndexMap::new(),
            is_sorted: false,
        }
    }

    pub fn sort(&mut self) {
        if !self.is_sorted {
            self.pokemon.sort_by(|_, a, _, b| {
                b.final_stats()
                    .deviations
                    .partial_cmp(&a.final_stats().deviations)
                    .unwrap()
            });
        }
    }

    pub fn process_json(
        min_elo: u64,
        json: &Vec<Option<&[u8]>>,
    ) -> Result<Vec<GameResult>, StatsError> {
        // ELO check
        for elo_bytes in [json[0], json[3]].iter() {
            if let Some(rating) = elo_bytes {
                match String::from_utf8_lossy(rating).parse::<f64>() {
                    Ok(n) => {
                        if (n as u64) < min_elo {
                            return Ok(vec![]);
                        }
                    }
                    Err(_) => {
                        return Ok(vec![]);
                    }
                };
            } else {
                return Ok(vec![]);
            }
        }

        let mut results = vec![];

        let mut json_parser =
            pikkr_annika::Pikkr::new(&vec!["$.species".as_bytes()], crate::PIKKR_TRAINING_ROUNDS)
                .unwrap();
        // see src/main.rs:44 for documentation on these magic numbers
        // (indices of parsed JSON)
        for team_idx in [1, 4].iter() {
            // json[16] = the winner
            let won = json[6] == json[team_idx + 1];

            let team_json = match json[*team_idx] {
                Some(team_bytes) => String::from_utf8_lossy(team_bytes),
                None => continue,
            };
            for pkmn_json in team_json
                .strip_prefix("[{")
                .unwrap()
                .strip_suffix("}]")
                .unwrap()
                .split("},{")
            {
                let species = match json_parser.parse(&["{", pkmn_json, "}"].join(""))?.get(0) {
                    Some(s) => match s {
                        Some(bytes) => Stats::normalize_species(&String::from_utf8_lossy(bytes)),
                        None => continue,
                    },
                    None => continue,
                };
                results.push(GameResult { species, won });
            }
        }
        Ok(results)
    }

    pub fn add_game_results(&mut self, results: Vec<GameResult>) {
        if results.is_empty() {
            return;
        }

        self.is_sorted = false; // we're adding data so it isn't sorted anymore
        for result in results {
            let wins = if result.won { 1 } else { 0 };
            match self.pokemon.get_mut(&result.species) {
                Some(s) => {
                    s.wins += wins;
                    s.games += 1;
                }
                None => {
                    self.pokemon
                        .insert(result.species, PokemonStats { games: 1, wins });
                }
            };
        }
    }

    fn normalize_species(species: &str) -> String {
        if species.starts_with("\"Pikachu-") {
            String::from("\"Pikachu\"")
        } else if species.starts_with("\"Unown-") {
            String::from("\"Unown\"")
        } else if species == "\"Gastrodon-East" {
            String::from("Gastrodon\"")
        } else if species == "\"Magearna-Original" {
            String::from("\"Magearna\"")
        } else if species == "\"Genesect-Douse" {
            String::from("\"Genesect\"")
        } else if species.starts_with("\"Basculin-") {
            String::from("\"Basculin\"")
        } else if species.starts_with("\"Sawsbuck-") {
            String::from("\"Sawsbuck\"")
        } else if species.starts_with("\"Vivillon-") {
            String::from("\"Vivillon\"")
        } else if species.starts_with("\"Florges-") {
            String::from("\"Florges\"")
        } else if species.starts_with("\"Furfrou-") {
            String::from("\"Furfrou\"")
        } else if species.starts_with("\"Minior-") {
            String::from("\"Minior\"")
        } else if species.starts_with("\"Gourgeist-") {
            String::from("\"Gourgeist\"")
        } else if species.starts_with("\"Toxtricity-") {
            String::from("\"Toxtricity\"")
        } else {
            species.to_string()
        }
    }
}

impl Output for Stats {
    fn to_csv(&mut self) -> String {
        self.sort();

        Itertools::intersperse(
            self.pokemon.iter().map(|(pokemon, stats)| {
                let fstats = stats.final_stats();
                [
                    pokemon.to_string(),
                    stats.games.to_string(),
                    stats.wins.to_string(),
                    fstats.winrate.to_string(),
                    fstats.deviations.to_string(),
                ]
                .join(",")
            }),
            String::from("\n"),
        )
        .collect()
    }

    fn to_human_readable(&mut self) -> String {
        let mut table = table!(["Rank", "Pokemon", "Deviations", "Winrate", "Games", "Wins"]);
        let mut cur_rank = 1;

        self.sort();

        for (pokemon, stats) in &self.pokemon {
            let fstats = stats.final_stats();

            let deviations = fstats.deviations.to_string();
            let mut winrate = fstats.winrate.to_string();
            winrate.push('%');

            table.add_row(row![
                cur_rank,
                pokemon.as_str()[1..pokemon.len() - 1],
                deviations,
                winrate,
                stats.games,
                stats.wins
            ]);
            cur_rank += 1;
        }

        table.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use lazy_static::lazy_static;
    use test::Bencher;

    lazy_static! {
        static ref SAMPLE_JSON: Vec<Option<&'static [u8]>> = vec![
            Some("1100".as_bytes()),
            Some(r#"[{"name":"gallant's pear","species":"Orbeetle","item":"Life Orb","ability":"Armor Time","moves":["Bug Buzz","Nasty Plot","Snipe Shot","King Giri Giri Slash"],"nature":"Timid","gender":"M","evs":{"hp":252,"atk":0,"def":4,"spa":0,"spd":0,"spe":252},"ivs":{"hp":31,"atk":31,"def":31,"spa":31,"spd":31,"spe":31},"level":100,"happiness":255,"shiny":false},{"name":"Akir","species":"Forretress","item":"Leftovers","ability":"Fortifications","moves":["Stealth Rock","Rapid Spin","U-turn","Ravelin"],"nature":"Impish","gender":"M","evs":{"hp":248,"atk":0,"def":252,"spa":0,"spd":0,"spe":8},"ivs":{"hp":31,"atk":31,"def":31,"spa":0,"spd":31,"spe":31},"level":100,"happiness":255,"shiny":false},{"name":"brouha","species":"Mantine","item":"Leftovers","ability":"Turbulence","moves":["Scald","Recover","Haze","Kinetosis"],"nature":"Calm","gender":"M","evs":{"hp":248,"atk":0,"def":8,"spa":0,"spd":252,"spe":0},"ivs":{"hp":31,"atk":0,"def":31,"spa":31,"spd":31,"spe":31},"level":100,"happiness":255,"shiny":false},{"name":"Kalalokki","species":"Wingull","item":"Kalalokkium Z","ability":"Magic Guard","moves":["Tailwind","Healing Wish","Encore","Blackbird"],"nature":"Timid","gender":"M","evs":{"hp":0,"atk":0,"def":0,"spa":252,"spd":4,"spe":252},"ivs":{"hp":31,"atk":0,"def":31,"spa":31,"spd":31,"spe":31},"level":100,"happiness":255,"shiny":false},{"name":"OM~!","species":"Glastrier","item":"Heavy Duty Boots","ability":"Filter","moves":["Stealth Rock","Recover","Earthquake","OM Zoom"],"nature":"Relaxed","gender":"M","evs":{"hp":252,"atk":0,"def":252,"spa":0,"spd":4,"spe":0},"ivs":{"hp":31,"atk":31,"def":31,"spa":31,"spd":31,"spe":0},"level":100,"happiness":255,"shiny":false},{"name":"vivalospride","species":"Darmanitan-Zen","item":"Heavy Duty Boots","ability":"Regenerator","moves":["Teleport","Toxic","Future Sight","DRIP BAYLESS"],"nature":"Modest","gender":"M","evs":{"hp":252,"atk":0,"def":4,"spa":252,"spd":0,"spe":0},"ivs":{"hp":31,"atk":0,"def":31,"spa":31,"spd":31,"spe":31},"level":100,"happiness":255,"shiny":false}]"#.as_bytes()),
            Some("annika".as_bytes()),
            Some("1400".as_bytes()),
            Some(r#"[{"name":"gallant's pear","species":"Orbeetle","item":"Life Orb","ability":"Armor Time","moves":["Bug Buzz","Nasty Plot","Snipe Shot","King Giri Giri Slash"],"nature":"Timid","gender":"M","evs":{"hp":252,"atk":0,"def":4,"spa":0,"spd":0,"spe":252},"ivs":{"hp":31,"atk":31,"def":31,"spa":31,"spd":31,"spe":31},"level":100,"happiness":255,"shiny":false},{"name":"Akir","species":"Forretress","item":"Leftovers","ability":"Fortifications","moves":["Stealth Rock","Rapid Spin","U-turn","Ravelin"],"nature":"Impish","gender":"M","evs":{"hp":248,"atk":0,"def":252,"spa":0,"spd":0,"spe":8},"ivs":{"hp":31,"atk":31,"def":31,"spa":0,"spd":31,"spe":31},"level":100,"happiness":255,"shiny":false},{"name":"brouha","species":"Mantine","item":"Leftovers","ability":"Turbulence","moves":["Scald","Recover","Haze","Kinetosis"],"nature":"Calm","gender":"M","evs":{"hp":248,"atk":0,"def":8,"spa":0,"spd":252,"spe":0},"ivs":{"hp":31,"atk":0,"def":31,"spa":31,"spd":31,"spe":31},"level":100,"happiness":255,"shiny":false},{"name":"Kalalokki","species":"Wingull","item":"Kalalokkium Z","ability":"Magic Guard","moves":["Tailwind","Healing Wish","Encore","Blackbird"],"nature":"Timid","gender":"M","evs":{"hp":0,"atk":0,"def":0,"spa":252,"spd":4,"spe":252},"ivs":{"hp":31,"atk":0,"def":31,"spa":31,"spd":31,"spe":31},"level":100,"happiness":255,"shiny":false},{"name":"OM~!","species":"Glastrier","item":"Heavy Duty Boots","ability":"Filter","moves":["Stealth Rock","Recover","Earthquake","OM Zoom"],"nature":"Relaxed","gender":"M","evs":{"hp":252,"atk":0,"def":252,"spa":0,"spd":4,"spe":0},"ivs":{"hp":31,"atk":31,"def":31,"spa":31,"spd":31,"spe":0},"level":100,"happiness":255,"shiny":false},{"name":"vivalospride","species":"Darmanitan-Zen","item":"Heavy Duty Boots","ability":"Regenerator","moves":["Teleport","Toxic","Future Sight","DRIP BAYLESS"],"nature":"Modest","gender":"M","evs":{"hp":252,"atk":0,"def":4,"spa":252,"spd":0,"spe":0},"ivs":{"hp":31,"atk":0,"def":31,"spa":31,"spd":31,"spe":31},"level":100,"happiness":255,"shiny":false}]"#.as_bytes()),
            Some("rust haters".as_bytes()),
            Some("annika".as_bytes()),
        ];
    }

    fn add_records(stats: &mut Stats, num: u32) {
        for _ in 0..num {
            let s = Stats::process_json(1050, &SAMPLE_JSON).unwrap();
            stats.add_game_results(s);
        }
    }

    #[bench]
    pub fn bench_process_json(b: &mut Bencher) {
        b.iter(|| Stats::process_json(1050, &SAMPLE_JSON));
    }

    #[bench]
    pub fn bench_process_and_add_json(b: &mut Bencher) {
        let mut stats = Stats::new();
        b.iter(|| {
            let s = Stats::process_json(1050, &SAMPLE_JSON).unwrap();
            stats.add_game_results(s);
        });
    }

    #[bench]
    pub fn bench_to_csv_10k(b: &mut Bencher) {
        let mut stats = Stats::new();
        add_records(&mut stats, 10000);
        b.iter(|| stats.to_csv());
    }

    #[bench]
    pub fn bench_to_prettytable_10k(b: &mut Bencher) {
        let mut stats = Stats::new();
        add_records(&mut stats, 10000);
        b.iter(|| stats.to_human_readable());
    }
}
