use std::{
    error::Error,
    time::{Duration, Instant},
};

use braille::BRAILLE;
use rand::Rng;
use termwiz::{
    caps::Capabilities,
    input::{MouseButtons, MouseEvent},
    surface::{Change, CursorVisibility},
    terminal::{buffered::BufferedTerminal, new_terminal, Terminal},
};

pub trait Frontend {
    fn run(&mut self, game: &mut GameOfLife) -> Result<(), Box<dyn Error>>;
}

pub struct BrailleRenderer<T: Terminal> {
    screen: BufferedTerminal<T>,
}

impl<T: Terminal> BrailleRenderer<T> {
    pub fn new(screen: BufferedTerminal<T>) -> Result<Self, Box<dyn Error>> {
        Ok(Self { screen })
    }

    pub fn size(&self) -> (usize, usize) {
        let (w, h) = self.screen.dimensions();
        (w * 2, h * 3)
    }

    pub fn screen(&mut self) -> &mut BufferedTerminal<T> {
        &mut self.screen
    }

    pub fn render(&mut self, game: &GameOfLife) {
        if self.screen.dimensions() != { (game.size.0 / 2, game.size.1 / 3) } {
            self.screen.resize(game.size.0, game.size.1);
        }
        // 2x3 groups of cells to be represented by braille chars
        let mut groups = vec![
            vec![[false, false, false, false, false, false]; game.size.0 / 2];
            game.size.1 / 3
        ];

        for y in 0..game.size.1 {
            for x in 0..game.size.0 {
                groups[y.saturating_div(3)][x.saturating_div(2)][(y % 3) + (x % 2)] =
                    game.get(x, y);
            }
        }

        let chars = groups
            .into_iter()
            .map(|row| {
                row.into_iter()
                    .map(|cell| {
                        BRAILLE[cell[0] as usize][cell[3] as usize][cell[1] as usize]
                            [cell[4] as usize][cell[2] as usize][cell[5] as usize][0][0]
                    })
                    .collect::<Vec<_>>()
            })
            .collect::<Vec<_>>();

        self.screen.add_change(Change::CursorPosition {
            x: termwiz::surface::Position::Absolute(0),
            y: termwiz::surface::Position::Absolute(0),
        });
        let mut buf = String::new();
        for y in 0..game.size.1 / 3 {
            buf.clear();
            for x in 0..game.size.0 / 2 {
                let char = chars[y][x];
                buf.push(char);
            }
            self.screen.add_change(&buf);
        }
        self.screen.flush().ok();
    }
}

impl<T: Terminal> Frontend for BrailleRenderer<T> {
    fn run(&mut self, game: &mut GameOfLife) -> Result<(), Box<dyn Error>> {
        let term = self.screen().terminal();
        term.enter_alternate_screen()?;
        term.set_raw_mode()?;
        self.screen
            .add_change(Change::CursorVisibility(CursorVisibility::Hidden));
        let mut start = Instant::now();
        loop {
            if start.elapsed() >= DELAY {
                start = Instant::now();
            }
            match self.screen.terminal().poll_input(Some(DELAY)) {
                Ok(res) => match res {
                    Some(evt) => match evt {
                        termwiz::input::InputEvent::Key(k) => {
                            if k.key == termwiz::input::KeyCode::Char('q') {
                                break;
                            }
                        }
                        termwiz::input::InputEvent::Resized { cols, rows } => {
                            *game = GameOfLife::new((cols as usize * 2, rows as usize * 3));
                            continue;
                        }
                        termwiz::input::InputEvent::Wake => {
                            continue;
                        }
                        termwiz::input::InputEvent::Mouse(MouseEvent {
                            x,
                            y,
                            mouse_buttons,
                            ..
                        }) => {
                            if mouse_buttons.contains(MouseButtons::LEFT) {
                                let col = x as usize * 2;
                                let row = y as usize * 3;
                                for y in 0..3 {
                                    for x in 0..2 {
                                        game.set(col + x, row + y, true);
                                    }
                                }
                            }
                        }
                        _ => {}
                    },
                    None => {}
                },
                Err(_) => {
                    break;
                }
            }
            if start.elapsed() <= DELAY {
                continue;
            }
            game.step();
            self.render(&game);
            self.screen.flush()?;
        }

        self.screen.terminal().exit_alternate_screen()?;
        self.screen
            .add_change(Change::CursorVisibility(CursorVisibility::Visible));
        Ok(())
    }
}

pub struct GameOfLife {
    size: (usize, usize),
    grid: Vec<bool>,
}

impl GameOfLife {
    pub fn new(size: (usize, usize)) -> Self {
        let mut new = Self {
            size,
            grid: vec![false; size.0 * size.1],
        };
        new.init();
        new
    }

    pub fn init(&mut self) {
        let mut rng = rand::thread_rng();
        for y in 0..self.size.1 {
            for x in 0..self.size.0 {
                self.set(x, y, rng.gen_bool(0.7));
            }
        }
    }

    pub fn size(&self) -> (usize, usize) {
        self.size
    }

    pub fn surface(&self) -> &[bool] {
        &self.grid
    }

    pub fn set(&mut self, x: usize, y: usize, value: bool) {
        self.grid[x + y * self.size.0] = value;
    }

    pub fn get(&self, x: usize, y: usize) -> bool {
        self.grid[x + y * self.size.0]
    }

    pub fn count_neighbors(&self, x: usize, y: usize) -> usize {
        let mut count = 0;
        for i in x.saturating_sub(1)..=(x + 1).min(self.size.0 - 1) {
            for j in y.saturating_sub(1)..=(y + 1).min(self.size.1 - 1) {
                if i == x && j == y {
                    continue;
                }
                if self.get(i, j) {
                    count += 1;
                }
            }
        }
        count
    }

    pub fn step(&mut self) {
        let mut next = self.grid.clone();
        for y in 0..self.size.1 {
            for x in 0..self.size.0 {
                let neighbors = self.count_neighbors(x, y);
                let cell = self.get(x, y);
                let next_cell = match (cell, neighbors) {
                    (true, 2) | (true, 3) => true,
                    (true, _) => false,
                    (false, 3) => true,
                    (false, _) => false,
                };
                next[x + y * self.size.0] = next_cell;
            }
        }
        self.grid = next;
    }
}

const DELAY: Duration = Duration::from_millis(50);

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let caps = Capabilities::new_from_env()?;
    let screen = BufferedTerminal::new(new_terminal(caps)?)?;
    let mut render = BrailleRenderer::new(screen)?;
    let mut game = GameOfLife::new(render.size());

    render.run(&mut game)?;

    Ok(())
}

#[test]
fn test_count_neighbors() {
    let mut game = GameOfLife::new((3, 3));
    game.set(0, 0, true);
    game.set(1, 0, true);
    game.set(2, 0, true);

    game.set(0, 1, true);
    game.set(1, 1, true);
    game.set(2, 1, true);
    let neighbors = GameOfLife::count_neighbors(&game, 1, 1);
    assert_eq!(neighbors, 5);
}
