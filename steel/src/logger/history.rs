use crate::logger::Input;
use std::{borrow::Cow, collections::VecDeque, io::Result};
use tokio::{fs, io::AsyncWriteExt};

pub struct History {
    pub path: &'static str,
    pub values: VecDeque<Cow<'static, str>>,
    pub pos: usize,
}
impl History {
    pub async fn new(path: &'static str) -> Self {
        let file_path = format!("{path}/history.txt");
        let values = if let Ok(true) = fs::try_exists(&file_path).await {
            let history = fs::read_to_string(file_path).await;
            let Ok(history) = history else {
                panic!("Cannot load history.");
            };
            history
                .split('\n')
                .map(|str| Cow::Owned(str.to_string()))
                .collect()
        } else {
            VecDeque::new()
        };
        History {
            path,
            values,
            pos: 0,
        }
    }
}
impl Input {
    pub fn add_history(&mut self) {
        if !self.history.values.is_empty() && self.history.values[0] == self.text {
            return;
        }
        self.history
            .values
            .push_front(Cow::Owned(self.text.clone()));
    }
    pub fn move_history(&mut self, dir: i8) -> Result<()> {
        self.history.pos = if dir < 0 && self.history.pos != 0 {
            self.history.pos - (-dir as usize)
        } else if dir > 0 && self.history.pos != self.history.values.len() {
            self.history.pos + dir as usize
        } else {
            self.history.pos
        };
        if self.history.pos == 0 {
            self.reset()?;
            return Ok(());
        }
        let text = self.history.values[self.history.pos - 1].clone();
        self.text = text.to_string();
        let length = text.chars().count();
        self.update_suggestion_list(length);
        self.rewrite_input(length, length)?;
        Ok(())
    }
    pub async fn save_history(&self) -> Result<()> {
        fs::create_dir_all(self.history.path).await?;
        let path = format!("{}/history.txt", self.history.path);
        if let Ok(true) = fs::try_exists(&path).await {
            fs::remove_file(&path).await?;
        }
        let mut file = fs::File::create_new(&path).await?;
        for line in &self.history.values {
            file.write_all(format!("{line}\n").as_bytes()).await?;
        }
        file.set_len(file.metadata().await?.len() - 1).await?;
        Ok(())
    }
}
