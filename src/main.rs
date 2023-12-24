//! Wave function collapse algorithm written in Rust.

use imgproc_rs::image::BaseImage;
use imgproc_rs::io;
use rand::distributions::{Distribution, Uniform};
use std::collections::{HashMap, HashSet};
use std::env;
use strum::IntoEnumIterator;
use strum_macros::{Display, EnumIter};

#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq, EnumIter)]
enum Direction {
    Up,
    Down,
    Left,
    Right,
}

impl Direction {
    pub const fn get_deltas(self) -> (i8, i8) {
        match self {
            Self::Up => (0, -1), // increasing rows go from top to bottom, so we flip signs for up and down
            Self::Down => (0, 1),
            Self::Left => (-1, 0),
            Self::Right => (1, 0),
        }
    }

    pub const fn get_opposite(self) -> Self {
        match self {
            Self::Up => Self::Down,
            Self::Down => Self::Up,
            Self::Left => Self::Right,
            Self::Right => Self::Left,
        }
    }
}

const WATER: [u8; 3] = [63, 72, 204];
const COAST: [u8; 3] = [255, 201, 14];
const GRASS: [u8; 3] = [34, 177, 76];

#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq, EnumIter, Display)]
enum TileType {
    Invalid,
    Coast,
    Grass,
    Water,
}

impl TileType {
    pub const fn from_pixel(pixel: [u8; 3]) -> Option<Self> {
        match pixel {
            WATER => Some(Self::Water),
            COAST => Some(Self::Coast),
            GRASS => Some(Self::Grass),
            _ => None,
        }
    }
}

#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq)]
struct Rule {
    pub from: TileType,
    pub to: TileType,
    pub direction: Direction,
}

impl Rule {
    pub const fn new(from: TileType, to: TileType, direction: Direction) -> Self {
        Self {
            from,
            to,
            direction,
        }
    }

    pub const fn reverse(from: TileType, to: TileType, direction: Direction) -> Self {
        Self {
            from: to,
            to: from,
            direction: direction.get_opposite(),
        }
    }
}

#[derive(Debug)]
struct Generation {
    pub ruleset: HashSet<Rule>,
    pub frequencies: HashMap<TileType, u32>,
}

fn get_all_tile_types() -> HashSet<TileType> {
    TileType::iter().collect::<HashSet<_>>()
}

fn add_adjacent_rules(
    ruleset: &mut HashSet<Rule>,
    image: &dyn BaseImage<u8>,
    w: u32,
    h: u32,
    rotate_rules: bool,
) {
    let (width, height) = image.info().wh();

    let from_pixel = image.get_pixel(w, h);
    let from = TileType::from_pixel([from_pixel[0], from_pixel[1], from_pixel[2]]).unwrap();

    for direction in Direction::iter() {
        let (del_w, del_h) = direction.get_deltas();

        let new_w = del_w + i8::try_from(w).unwrap();
        let new_h = del_h + i8::try_from(h).unwrap();

        if new_w < 0 || new_h < 0 {
            continue;
        }

        let new_w = u32::try_from(new_w).unwrap();
        let new_h = u32::try_from(new_h).unwrap();

        if (0..width).contains(&new_w) && (0..height).contains(&new_h) {
            let to_pixel = image.get_pixel(new_w, new_h);
            let to = TileType::from_pixel([to_pixel[0], to_pixel[1], to_pixel[2]]).unwrap();

            ruleset.insert(Rule::new(from, to, direction));
            ruleset.insert(Rule::reverse(from, to, direction));

            if rotate_rules {
                for direction in Direction::iter() {
                    ruleset.insert(Rule::new(from, to, direction));
                    ruleset.insert(Rule::reverse(from, to, direction));
                }
            }
        }
    }
}

fn update_frequencies(
    frequencies: &mut HashMap<TileType, u32>,
    image: &dyn BaseImage<u8>,
    w: u32,
    h: u32,
) {
    let from_pixel = image.get_pixel(w, h);
    let tile = TileType::from_pixel([from_pixel[0], from_pixel[1], from_pixel[2]]).unwrap();

    match frequencies.get_mut(&tile) {
        Some(result) => {
            *result = *result + 1;
        }
        None => {
            frequencies.insert(tile, 1);
        }
    }
}

fn generation_init(input_path: &str, rotate_rules: bool) -> Generation {
    let mut ruleset = HashSet::<Rule>::new();
    let mut frequencies = HashMap::<TileType, u32>::new();

    let image = io::read(input_path).unwrap();

    let (width, height) = image.info().wh();
    for h in 0..height {
        for w in 0..width {
            add_adjacent_rules(&mut ruleset, &image, w, h, rotate_rules);
            update_frequencies(&mut frequencies, &image, w, h);
        }
    }

    for tile_type in TileType::iter() {
        if frequencies.get(&tile_type).is_none() {
            frequencies.insert(tile_type, 0);
        }
    }

    Generation {
        ruleset,
        frequencies,
    }
}

#[derive(Clone, Debug)]
struct PossibileTiles {
    pub choices: HashSet<TileType>,
}

#[derive(Clone, Debug)]
enum Tile {
    Revealed(TileType),
    Hidden(PossibileTiles),
}

impl Default for Tile {
    fn default() -> Self {
        Self::Hidden(PossibileTiles {
            choices: get_all_tile_types(),
        })
    }
}

fn choose_tile(possible_tiles: &PossibileTiles, frequencies: &HashMap<TileType, u32>) -> TileType {
    if possible_tiles.choices.is_empty() {
        return TileType::Invalid;
    }

    let mut tile_choices = vec![];
    for tile in &possible_tiles.choices {
        let frequency = frequencies.get(tile).unwrap();
        for _ in 0..(*frequency) {
            tile_choices.push(tile);
        }
    }
    let mut rng = rand::thread_rng();
    let distribution = Uniform::from(0..tile_choices.len());
    let index = distribution.sample(&mut rng);

    *tile_choices[index]
}

fn remove_choices(
    source_tile: TileType,
    direction: Direction,
    ruleset: &HashSet<Rule>,
    original_choices: &mut HashSet<TileType>,
) {
    let mut allowed_from_source = HashSet::<TileType>::new();
    for rule in ruleset {
        if rule.from == source_tile && rule.direction == direction {
            allowed_from_source.insert(rule.to);
        }
    }
    let choices_to_remove = &get_all_tile_types() - &allowed_from_source;
    *original_choices = &(*original_choices) - &choices_to_remove;
}

fn update_possible_tiles(
    board: &mut [Vec<Tile>],
    ruleset: &HashSet<Rule>,
    w: usize,
    h: usize,
    direction: Direction,
) {
    let Tile::Revealed(source_tile) = board[h][w] else {
        return;
    };

    let (del_w, del_h) = direction.get_deltas();

    let new_w = del_w + i8::try_from(w).unwrap();
    let new_h = del_h + i8::try_from(h).unwrap();

    if new_w < 0 || new_h < 0 {
        return;
    }

    if let Some(row) = board.get_mut(usize::try_from(new_h).unwrap()) {
        if let Some(cell) = row.get_mut(usize::try_from(new_w).unwrap()) {
            match cell {
                Tile::Hidden(possible_tiles) => {
                    remove_choices(source_tile, direction, ruleset, &mut possible_tiles.choices);
                }
                Tile::Revealed(_) => {}
            }
        }
    }
}

fn reveal(board: &mut [Vec<Tile>], generation: &Generation, w: usize, h: usize) {
    if let Some(row) = board.get_mut(h) {
        if let Some(tile) = row.get_mut(w) {
            if let Tile::Hidden(possible_tiles) = tile {
                let new_type = choose_tile(possible_tiles, &generation.frequencies);
                *tile = Tile::Revealed(new_type);

                for direction in Direction::iter() {
                    update_possible_tiles(board, &generation.ruleset, w, h, direction);
                }
            }
        }
    };
}

fn main() {
    let args: Vec<String> = env::args().collect();
    let file_name = &args[1];
    let file_path = format!("./resources/{file_name}");

    let generation = generation_init(&file_path, true);
    let mut board = vec![vec![Tile::default(); 20]; 20];

    for h in 0..board.len() {
        for w in 0..board[0].len() {
            reveal(&mut board, &generation, w, h);
        }
    }

    for h in 0..board.len() {
        for w in 0..board[0].len() {
            match board[h].get(w) {
                Some(Tile::Revealed(tile)) => match tile {
                    TileType::Invalid => print!("\u{1f7e5}"),
                    TileType::Coast => print!("\u{1f7e8}"),
                    TileType::Grass => print!("\u{1f7e9}"),
                    TileType::Water => print!("\u{1f7e6}"),
                },
                _ => print!("\u{2b1c}"),
            }
        }
        println!();
    }
}
