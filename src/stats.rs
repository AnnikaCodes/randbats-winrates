/// Stats code

use indexmap::IndexMap;
use itertools::Itertools;
use prettytable::*;

pub trait Output {
    fn to_human_readable(&mut self) -> String;
    fn to_csv(&mut self) -> String;
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
#[derive(Copy, Clone)]
struct PokemonStats {
    games: u32,
    wins: u32,
}

impl PokemonStats {
    // Approximate winrate * 10000
    fn winrate(&self) -> u32 {
        (self.wins * 10000) / self.games
    }
}

/// Stores overall statistics
pub struct Stats<'a> {
    /// Pokemon:statistics map
    pokemon: IndexMap<String, PokemonStats>,
    min_elo: u64,
    is_sorted: bool,
    team_json_parser: pikkr_annika::Pikkr<'a>,
}

impl<'a> Stats<'a> {
    pub fn new(min_elo: u64) -> Self {
        Self {
            min_elo,
            pokemon: IndexMap::new(),
            is_sorted: false,
            team_json_parser: pikkr_annika::Pikkr::new(&vec!["$.species".as_bytes()], crate::PIKKR_TRAINING_ROUNDS).unwrap(),
        }
    }

    pub fn sort(&mut self) {
        if !self.is_sorted {
            self.pokemon.sort_by(|_, a, _, b| b.winrate().cmp(&a.winrate()));
        }
    }

    pub fn process_json(&mut self, json: Vec<Option<&[u8]>>) -> Result<(), StatsError> {
        self.is_sorted = false; // we're adding data so it isn't sorted anymore

        // ELO check
        for elo_bytes in [json[0], json[3]].iter() {
            if let Some(rating) = elo_bytes {
                match String::from_utf8_lossy(rating).parse::<f64>() {
                    Ok(n) => {
                        if (n as u64) < self.min_elo { return Ok(()); }
                    },
                    Err(_) => {
                        return Ok(());
                    },
                };
            } else {
                return Ok(());
            }
        }

        // see src/main.rs:44 for documentation on these magic numbers
        // (indices of parsed JSON)
        for team_idx in [1, 4].iter() {
            // json[16] = the winner
            let wins = if json[6] == json[team_idx + 1] { 1 } else { 0 };

            let team_json = match json[*team_idx] {
                Some(team_bytes) => String::from_utf8_lossy(team_bytes),
                None => continue,
            };
            for pkmn_json in team_json.strip_prefix("[{").unwrap().strip_suffix("}]").unwrap().split("},{") {
                let species = match self.team_json_parser.parse(&["{", pkmn_json, "}"].join(""))?.get(0) {
                    Some(s) => match s {
                        Some(bytes) => Stats::normalize_species(&String::from_utf8_lossy(bytes)),
                        None => continue,
                    }
                    None => continue,
                };
                match self.pokemon.get_mut(&species) {
                    Some(s) => {
                        s.wins += wins;
                        s.games += 1;
                    },
                    None => {
                        self.pokemon.insert(species, PokemonStats { games: 1, wins });
                    }
                };
            }
        }
        Ok(())
    }

    fn normalize_species(species: &str) -> String {
        if species.starts_with("Pikachu-") {
            String::from("Pikachu")
        } else if species.starts_with("Unown-") {
            String::from("Unown")
        } else if species == "Gastrodon-East" {
            String::from("Gastrodon")
        } else if species == "Magearna-Original" {
            String::from("Magearna")
        } else if species == "Genesect-Douse" {
            String::from("Genesect")
        } else if species == "Basculin-Blue-Striped" {
            String::from("Basculin")
        } else if species.starts_with("Sawsbuck-") {
            String::from("Sawsbuck")
        } else if species.starts_with("Vivillon-") {
            String::from("Vivillon")
        } else if species.starts_with("Florges-") {
            String::from("Florges")
        } else if species.starts_with("Furfrou-") {
            String::from("Furfrou")
        } else if species.starts_with("Minior-") {
            String::from("Minior")
        } else if species.starts_with("Gourgeist-") {
            String::from("Gourgeist")
        } else if species.starts_with("Toxtricity-") {
            String::from("Toxtricity")
        } else {
            species.to_string()
        }
    }
}

impl<'a> Output for Stats<'a> {
    fn to_csv(&mut self) -> String {
        self.sort();

        self.pokemon
            .iter()
            .map(|(pokemon, stats)| {
                [
                    pokemon.to_string(),
                    stats.games.to_string(),
                    stats.wins.to_string(),
                    (stats.winrate() as f32 / 100.0).to_string(),
                ].join(",")
            })
            .intersperse(String::from("\n"))
            .collect()
    }

    fn to_human_readable(&mut self) -> String {
        let mut table = table!(["Rank", "Pokemon", "Winrate", "Games", "Wins"]);
        let mut cur_rank = 1;

        self.sort();

        for (pokemon, stats) in &self.pokemon {
            let mut winrate = (stats.winrate() as f32 / 100.0).to_string();
            winrate.push('%');

            table.add_row(row![cur_rank, pokemon.as_str()[1..pokemon.len() - 1], winrate, stats.games, stats.wins]);
            cur_rank += 1;
        }

        table.to_string()
    }
}
