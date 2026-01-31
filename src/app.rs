use crate::utils::error::Result;

pub struct App {
    pub should_quit: bool,
}

impl App {
    pub fn new() -> Result<Self> {
        Ok(Self {
            should_quit: false,
        })
    }

    pub fn quit(&mut self) {
        self.should_quit = true;
    }

    pub fn should_quit(&self) -> bool {
        self.should_quit
    }
}

impl Default for App {
    fn default() -> Self {
        Self {
            should_quit: false,
        }
    }
}
