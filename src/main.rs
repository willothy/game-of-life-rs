use std::time::Duration;

use braille::BRAILLE;
use rand::Rng;
use termwiz::{
    caps::Capabilities,
    surface::{Change, CursorVisibility, Surface},
    terminal::{buffered::BufferedTerminal, new_terminal, Terminal},
};

pub struct GameOfLife {
    size: (usize, usize),
    grid: Vec<bool>,
}

impl GameOfLife {
    pub fn new(mut size: (usize, usize)) -> Self {
        size.0 *= 2;
        size.1 *= 3;
        Self {
            size,
            grid: vec![false; size.0 * size.1],
        }
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

    pub fn render(&self, screen: &mut Surface) {
        if screen.dimensions() != { (self.size.0 / 2, self.size.1 / 3) } {
            screen.resize(self.size.0, self.size.1);
        }
        // 2x3 groups of cells to be represented by braille chars
        let mut groups = vec![
            vec![[false, false, false, false, false, false]; self.size.0 / 2];
            self.size.1 / 3
        ];

        for y in 0..self.size.1 {
            for x in 0..self.size.0 {
                let group = &mut groups[y.saturating_div(3)][x.saturating_div(2)];
                group[(y % 3) + (x % 2)] = self.get(x, y);
            }
        }

        let chars = groups
            .into_iter()
            .map(|row| {
                row.into_iter()
                    .map(|cell| {
                        let char = BRAILLE[cell[0] as usize][cell[3] as usize][cell[1] as usize]
                            [cell[4] as usize][cell[2] as usize][cell[5] as usize][0][0];
                        char
                    })
                    .collect::<Vec<_>>()
            })
            .collect::<Vec<_>>();

        screen.add_change(Change::CursorPosition {
            x: termwiz::surface::Position::Absolute(0),
            y: termwiz::surface::Position::Absolute(0),
        });
        let mut buf = String::new();
        for y in 0..self.size.1 / 3 {
            buf.clear();
            for x in 0..self.size.0 / 2 {
                let char = chars[y][x];
                buf.push(char);
            }
            screen.add_change(&buf);
        }
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let caps = Capabilities::new_from_env()?;
    let mut term = new_terminal(caps)?;
    term.enter_alternate_screen()?;
    term.set_raw_mode()?;
    let mut screen = BufferedTerminal::new(term)?;
    screen.add_change(Change::CursorVisibility(CursorVisibility::Hidden));
    let (w, h) = screen.dimensions();

    let mut game = GameOfLife::new((w, h));
    game.init();

    let mut surface = Surface::new(w, h);

    loop {
        match screen
            .terminal()
            .poll_input(Some(Duration::from_millis(50)))
        {
            Ok(res) => match res {
                Some(evt) => match evt {
                    termwiz::input::InputEvent::Key(k) => {
                        if k.key == termwiz::input::KeyCode::Char('q') {
                            break;
                        }
                    }
                    termwiz::input::InputEvent::Resized { cols, rows } => {
                        game = GameOfLife::new((cols as usize, rows as usize));
                        game.init();
                    }
                    _ => {}
                },
                None => {}
            },
            Err(_) => {
                break;
            }
        }
        game.step();
        game.render(&mut surface);
        screen.draw_from_screen(&surface, 0, 0);
        screen.flush()?;
    }

    screen.terminal().exit_alternate_screen()?;
    screen.add_change(Change::CursorVisibility(CursorVisibility::Visible));

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
