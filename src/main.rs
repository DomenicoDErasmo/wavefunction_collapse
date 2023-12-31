//! Wave function collapse algorithm written in Rust.

#![allow(clippy::missing_docs_in_private_items)]
#![allow(clippy::pattern_type_mismatch)]
#![allow(clippy::single_call_fn)]

use core::hash::BuildHasher;
use imgproc_rs::image::BaseImage;
use imgproc_rs::io;
use rand::distributions::{Distribution, Uniform};
use std::collections::hash_map::Entry;
use std::collections::{HashMap, HashSet};
use std::env;
use strum::IntoEnumIterator;
use strum_macros::EnumIter;

macro_rules! unwrap_result_or_return {
    ( $e:expr ) => {
        match $e {
            Ok(x) => x,
            Err(_) => return,
        }
    };
}

macro_rules! unwrap_option_or_return {
    ( $e:expr ) => {
        match $e {
            Some(x) => x,
            None => return,
        }
    };
}

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

#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq)]
struct TileType {
    pub rgb: [u8; 3],
}

const INVALID_TILE: TileType = TileType { rgb: [255, 0, 0] };
const WATER_TILE: TileType = TileType { rgb: [63, 72, 204] };
const COAST_TILE: TileType = TileType {
    rgb: [255, 201, 14],
};
const GRASS_TILE: TileType = TileType { rgb: [34, 177, 76] };

impl TileType {
    pub fn from_pixel(image: &dyn BaseImage<u8>, width: u32, height: u32) -> Option<Self> {
        let raw_pixel = image.get_pixel(width, height);
        let (Some(first), Some(second), Some(third)) =
            (raw_pixel.first(), raw_pixel.get(1), raw_pixel.get(2))
        else {
            println!("Pixel does not have three values.");
            return None;
        };

        Some(Self {
            rgb: [*first, *second, *third],
        })
    }
}

impl<'src, S> FromIterator<&'src TileType> for HashSet<TileType, S>
where
    S: BuildHasher + Default,
{
    fn from_iter<T: IntoIterator<Item = &'src TileType>>(iter: T) -> Self {
        iter.into_iter().copied().collect()
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

fn get_all_tile_types(ruleset: &HashSet<Rule>) -> HashSet<TileType> {
    let mut result = HashSet::<TileType>::new();

    for rule in ruleset {
        result.insert(rule.from);
        result.insert(rule.to);
    }

    result
}

fn add_adjacent_rules(
    ruleset: &mut HashSet<Rule>,
    image: &dyn BaseImage<u8>,
    width: u32,
    height: u32,
    rotate_rules: bool,
) {
    let (max_width, max_height) = image.info().wh();

    let from = unwrap_option_or_return!(TileType::from_pixel(image, width, height));

    for direction in Direction::iter() {
        let (del_w, del_h) = direction.get_deltas();

        let raw_new_w = del_w
            .checked_add(unwrap_result_or_return!(i8::try_from(width)))
            .unwrap_or(i8::MAX);
        let raw_new_h = del_h
            .checked_add(unwrap_result_or_return!(i8::try_from(height)))
            .unwrap_or(i8::MAX);

        if raw_new_w < 0 || raw_new_h < 0 {
            continue;
        }

        let new_w = unwrap_result_or_return!(u32::try_from(raw_new_w));
        let new_h = unwrap_result_or_return!(u32::try_from(raw_new_h));

        if (0..max_width).contains(&new_w) && (0..max_height).contains(&new_h) {
            let to = unwrap_option_or_return!(TileType::from_pixel(image, new_w, new_h));

            ruleset.insert(Rule::new(from, to, direction));
            ruleset.insert(Rule::reverse(from, to, direction));

            if rotate_rules {
                for rotate_direction in Direction::iter() {
                    ruleset.insert(Rule::new(from, to, rotate_direction));
                    ruleset.insert(Rule::reverse(from, to, rotate_direction));
                }
            }
        }
    }
}

fn update_frequencies(
    frequencies: &mut HashMap<TileType, u32>,
    image: &dyn BaseImage<u8>,
    width: u32,
    height: u32,
) {
    let tile = unwrap_option_or_return!(TileType::from_pixel(image, width, height));

    if let Entry::Vacant(entry) = frequencies.entry(tile) {
        entry.insert(1);
    } else {
        *unwrap_option_or_return!(frequencies.get_mut(&tile)) =
            unwrap_option_or_return!(frequencies.get_mut(&tile))
                .checked_add(1)
                .unwrap_or(u32::MAX);
    }
}

fn generation_init(input_path: &str, rotate_rules: bool) -> Option<Generation> {
    let mut ruleset = HashSet::<Rule>::new();
    let mut frequencies = HashMap::<TileType, u32>::new();

    let Ok(image) = io::read(input_path) else {
        return None;
    };

    let (max_width, max_height) = image.info().wh();
    for height in 0..max_height {
        for width in 0..max_width {
            add_adjacent_rules(&mut ruleset, &image, width, height, rotate_rules);
            update_frequencies(&mut frequencies, &image, width, height);
        }
    }

    for tile_type in get_all_tile_types(&ruleset) {
        if frequencies.get(&tile_type).is_none() {
            frequencies.insert(tile_type, 0);
        }
    }

    Some(Generation {
        ruleset,
        frequencies,
    })
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

fn choose_tile(possible_tiles: &PossibileTiles, frequencies: &HashMap<TileType, u32>) -> TileType {
    if possible_tiles.choices.is_empty() {
        return INVALID_TILE;
    }

    let mut tile_choices = vec![];
    for tile in &possible_tiles.choices {
        let frequency = frequencies.get(tile).unwrap_or(&0);
        for _ in 0..(*frequency) {
            tile_choices.push(tile);
        }
    }
    let mut rng = rand::thread_rng();
    let distribution = Uniform::from(0..tile_choices.len());
    let index = distribution.sample(&mut rng);

    **tile_choices.get(index).unwrap_or(&&INVALID_TILE)
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

    let all_tile_types = get_all_tile_types(ruleset);
    let choices_to_remove = all_tile_types.difference(&allowed_from_source);
    let collected_difference = choices_to_remove.collect::<HashSet<_>>();
    let new_choices = original_choices
        .difference(&collected_difference)
        .collect::<HashSet<_>>();
    *original_choices = new_choices;
}

fn update_possible_tiles(
    board: &mut [Vec<Tile>],
    ruleset: &HashSet<Rule>,
    width: usize,
    height: usize,
    direction: Direction,
) {
    let source_tile = if let Some(row) = board.get(height) {
        if let Some(&Tile::Revealed(tile)) = row.get(width) {
            tile
        } else {
            return;
        }
    } else {
        return;
    };

    let (del_w, del_h) = direction.get_deltas();

    let new_w = del_w
        .checked_add(unwrap_result_or_return!(i8::try_from(width)))
        .unwrap_or(i8::MAX);
    let new_h = del_h
        .checked_add(unwrap_result_or_return!(i8::try_from(height)))
        .unwrap_or(i8::MAX);

    if new_w < 0 || new_h < 0 {
        return;
    }

    if let Some(row) = board.get_mut(unwrap_result_or_return!(usize::try_from(new_h))) {
        if let Some(cell) = row.get_mut(unwrap_result_or_return!(usize::try_from(new_w))) {
            match cell {
                Tile::Hidden(possible_tiles) => {
                    remove_choices(source_tile, direction, ruleset, &mut possible_tiles.choices);
                }
                Tile::Revealed(_) => {}
            }
        }
    }
}

fn reveal(board: &mut [Vec<Tile>], generation: &Generation, width: usize, height: usize) {
    if let Some(row) = board.get_mut(height) {
        if let Some(tile) = row.get_mut(width) {
            if let Tile::Hidden(possible_tiles) = tile {
                let new_type = choose_tile(possible_tiles, &generation.frequencies);
                *tile = Tile::Revealed(new_type);

                for direction in Direction::iter() {
                    update_possible_tiles(board, &generation.ruleset, width, height, direction);
                }
            }
        }
    };
}

fn main() {
    let args: Vec<String> = env::args().collect();
    let Some(file_name) = args.get(1) else {
        println!("Must pass an argument specifying the file to use. Exiting.");
        return;
    };
    let file_path = format!("./resources/{file_name}");

    let Some(generation) = generation_init(&file_path, true) else {
        println!("Failed to create generation rules based on the provided file. Exiting.",);
        return;
    };

    let mut board = vec![
        vec![
            Tile::Hidden(PossibileTiles {
                choices: get_all_tile_types(&generation.ruleset)
            });
            20
        ];
        20
    ];

    let max_height = board.len();
    let empty_row = &vec![];
    let first_row = board.get(0).unwrap_or(empty_row);
    let max_width = first_row.len();

    for height in 0..max_height {
        for width in 0..max_width {
            reveal(&mut board, &generation, width, height);
        }
    }

    for height in 0..max_height {
        for width in 0..max_width {
            if let Some(row) = board.get(height) {
                match row.get(width) {
                    Some(&Tile::Revealed(tile)) => match tile {
                        COAST_TILE => print!("\u{1f7e8}"),
                        GRASS_TILE => print!("\u{1f7e9}"),
                        WATER_TILE => print!("\u{1f7e6}"),
                        _ => print!("\u{1f7e5}"),
                    },
                    _ => print!("\u{2b1c}"),
                }
            }
        }
        println!();
    }
}
