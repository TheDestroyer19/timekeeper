use chrono::{DateTime, Local};

/// A block of time
#[derive(serde::Deserialize, serde::Serialize)]
pub struct Block {
    pub start: DateTime<Local>,
    pub end: DateTime<Local>,
}

#[derive(Default)]
#[derive(serde::Deserialize, serde::Serialize)]
pub struct StopWatch {
    blocks: Vec<Block>,
    current: Option<Block>,
}

impl StopWatch {
    pub fn start(&mut self) {
        if self.current.is_none() {
            self.current = Some(Block {
                start: Local::now(),
                end: Local::now(),
            })
        } else {
            eprintln!("Called start on an already running stopwatch");
        }
    }

    pub fn stop(&mut self) {
        if let Some(mut block) = self.current.take() {
            block.end = Local::now();
            self.blocks.push(block);
        } else {
            eprintln!("Called stop on an already stopped stopwatch");
        }
    }

    pub fn current(&self) -> Option<&Block> {
        self.current.as_ref()
    }

    pub fn delete(&mut self, index: usize) {
        self.blocks.remove(index);
    }

    pub fn all_blocks(&self) -> impl Iterator<Item = (usize, &Block)> {
        self.blocks.iter().enumerate()
    }
}