use imgproc_rs::io;
use strum::IntoEnumIterator;
use std::collections::{HashSet, HashMap};
use imgproc_rs::image::BaseImage;
use strum_macros::EnumIter;

#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq, EnumIter)]
enum Direction {
    Up,
    Down,
    Left,
    Right,
}

impl Direction {
    pub fn get_deltas(&self) -> (i8, i8) {
        match self {
            Self::Up => (0, -1), // increasing rows go from top to bottom, so we flip signs for up and down
            Self::Down => (0, 1),
            Self::Left => (-1, 0),
            Self::Right => (1, 0),
        }
    }

    pub fn get_opposite(&self) -> Self {
        match self {
            Self::Up => Self::Down,
            Self::Down => Self::Up,
            Self::Left => Self::Right,
            Self::Right => Self::Left
        }
    }
}

const WATER: [u8; 3] = [ 63,  72, 204];
const COAST: [u8; 3] = [255, 201,  14];
const GRASS: [u8; 3] = [ 34, 177,  76];

#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq)]
enum Tile {
    Coast,
    Grass,
    Water,
}

impl Tile {
    pub fn from_pixel(pixel: &[u8; 3]) -> Option<Self> {
        match *pixel {
            WATER => Some(Self::Water),
            COAST => Some(Self::Coast),
            GRASS => Some(Self::Grass),
            _ => None,
        }
    }
}

#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq)]
struct Rule {
    pub from: Tile,
    pub to: Tile,
    pub direction: Direction,
}

impl Rule {
    pub fn new(from: Tile, to: Tile, direction: Direction) -> Self {
        Self {
            from,
            to,
            direction,
        }
    }

    pub fn reverse(from: Tile, to: Tile, direction: Direction) -> Self {
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
    pub frequencies: HashMap<Tile, u32>,
    pub num_tiles: u32,
}

fn add_adjacent_rules(ruleset: &mut HashSet<Rule>, image: &dyn BaseImage<u8>, w: u32, h: u32) {
    let (width, height) = image.info().wh();

    let from_pixel = image.get_pixel(w, h);
    let from = Tile::from_pixel(&[from_pixel[0], from_pixel[1], from_pixel[2]]).unwrap();

    for direction in Direction::iter() {
        let (del_w, del_h) = direction.get_deltas();

        let new_w = del_w + w as i8;
        let new_h = del_h + h as i8;


        if (0..width).contains(&(new_w as u32)) 
            && (0..height).contains(&(new_h as u32)) {
                let to_pixel = image.get_pixel(new_w as u32, new_h as u32);
                let to = Tile::from_pixel(&[to_pixel[0], to_pixel[1], to_pixel[2]]).unwrap();

            ruleset.insert(Rule::new(from, to, direction));
            ruleset.insert(Rule::reverse(from, to, direction));
        }
    }
}

fn update_frequencies(frequencies: &mut HashMap<Tile, u32>, image: &dyn BaseImage<u8>, w: u32, h: u32) {
    let from_pixel = image.get_pixel(w, h);
    let tile = Tile::from_pixel(&[from_pixel[0], from_pixel[1], from_pixel[2]]).unwrap();
    match frequencies.get_mut(&tile) {
        Some(result) => {
            *result = *result + 1;
        },
        None => {
            frequencies.insert(tile, 1);
        }
    }
}

fn init(input_path: &str) -> Generation {
    let mut ruleset = HashSet::<Rule>::new();
    let mut frequencies = HashMap::<Tile, u32>::new();

    let image = io::read(input_path).unwrap();

    let (width, height) = image.info().wh();
    for h in 0..height {
        for w in 0..width {
            add_adjacent_rules(&mut ruleset, &image, w, h);
            update_frequencies(&mut frequencies, &image, w, h);
        }
    }

    let num_tiles = frequencies.values().cloned().sum();

    Generation {ruleset, frequencies, num_tiles}
}

fn main() {
    let generation = init("resources/beach.bmp");
    println!("{:#?}", generation);
}