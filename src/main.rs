use imgproc_rs::io;
use strum::IntoEnumIterator;
use std::collections::HashSet;
use imgproc_rs::image::BaseImage;
use strum_macros::EnumIter;

const WATER: [u8; 3] = [ 63,  72, 204];
const SAND: [u8; 3] = [255, 201,  14];
const GRASS: [u8; 3] = [ 34, 177,  76];

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
            Self::Up => (0, 1),
            Self::Down => (0, -1),
            Self::Left => (-1, 0),
            Self::Right => (0, 1),
        }
    }
}

#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq)]
enum Tile {
    Sand,
    Grass,
    Water,
}

impl Tile {
    pub fn from_pixel(pixel: &[u8; 3]) -> Option<Self> {
        match *pixel {
            WATER => Some(Self::Water),
            SAND => Some(Self::Sand),
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

            ruleset.insert(Rule {from, to, direction});
        }
    }
}

// TODO: verify and fix
fn init(input_path: &str) -> HashSet<Rule> {
    let mut ruleset = HashSet::<Rule>::new();
    let image = io::read(input_path).unwrap();

    let (width, height) = image.info().wh();
    for w in 0..width {
        for h in 0..height {
            add_adjacent_rules(&mut ruleset, &image, w, h);
        }
    }

    ruleset
}

fn main() {
    let ruleset = init("resources/beach.bmp");
    println!("{:#?}", ruleset);
}