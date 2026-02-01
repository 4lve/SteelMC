use std::ops::Range;

pub struct Selection {
    anchor: usize,
    active: usize,
}

impl Selection {
    pub fn new() -> Self {
        Selection {
            anchor: 0,
            active: 0,
        }
    }

    pub fn is_active(&self) -> bool {
        self.anchor != self.active
    }

    pub fn get_range(&self) -> Range<usize> {
        if self.anchor <= self.active {
            self.anchor..self.active
        } else {
            self.active..self.anchor
        }
    }

    pub fn clear(&mut self) {
        self.anchor = 0;
        self.active = 0;
    }

    pub fn set(&mut self, anchor: usize, active: usize) {
        self.anchor = anchor;
        self.active = active;
    }

    pub fn extend(&mut self, new_active: usize) {
        self.active = new_active;
    }

    pub fn start_at(&mut self, pos: usize) {
        self.anchor = pos;
        self.active = pos;
    }
}
