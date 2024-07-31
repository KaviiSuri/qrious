use crate::viz::Visualizer;
use anyhow::{anyhow, Result};

#[derive(PartialEq, Debug, Clone)]
pub struct Rect {
    pub top: f32,
    pub bottom: f32,
    pub left: f32,
    pub right: f32,
}

impl Rect {
    #[allow(dead_code)]
    pub fn from_center_and_size(cx: f32, cy: f32, width: f32, height: f32) -> Self {
        Self {
            top: cy - height / 2.0,
            bottom: cy + height / 2.0,
            left: cx - width / 2.0,
            right: cx + width / 2.0,
        }
    }

    #[allow(dead_code)]
    pub fn from_corners(left: f32, top: f32, right: f32, bottom: f32) -> Self {
        Self {
            top,
            bottom,
            left,
            right,
        }
    }

    #[allow(dead_code)]
    pub fn to_corners(&self) -> (f32, f32, f32, f32) {
        (self.left, self.top, self.right, self.bottom)
    }

    #[allow(dead_code)]
    pub fn to_center_and_size(&self) -> (f32, f32, f32, f32) {
        (
            (self.left + self.right) / 2.0,
            (self.top + self.bottom) / 2.0,
            self.right - self.left,
            self.bottom - self.top,
        )
    }

    #[allow(dead_code)]
    pub fn cx(&self) -> f32 {
        (self.left + self.right) / 2.0
    }

    #[allow(dead_code)]
    pub fn cy(&self) -> f32 {
        (self.top + self.bottom) / 2.0
    }

    #[allow(dead_code)]
    pub fn width(&self) -> f32 {
        self.right - self.left
    }

    #[allow(dead_code)]
    pub fn height(&self) -> f32 {
        self.bottom - self.top
    }

    #[allow(dead_code)]
    pub fn top(&self) -> f32 {
        self.top
    }
    #[allow(dead_code)]
    pub fn bottom(&self) -> f32 {
        self.bottom
    }
    #[allow(dead_code)]
    pub fn left(&self) -> f32 {
        self.left
    }
    #[allow(dead_code)]
    pub fn right(&self) -> f32 {
        self.right
    }

    #[allow(dead_code)]
    pub fn draw(&self, viz: &mut Visualizer, color: &str, fill: Option<&str>) -> Result<()> {
        viz.draw_rect(
            self.cx(),
            self.cy(),
            self.width(),
            self.height(),
            color,
            fill,
        )?;
        Ok(())
    }
}

pub trait IteratorExt: Iterator {
    fn take_or_err(&mut self, n: usize) -> Result<Vec<Self::Item>>
    where
        Self: Sized,
    {
        let mut result = Vec::with_capacity(n);

        for _ in 0..n {
            match self.next() {
                Some(item) => result.push(item),
                None => return Err(anyhow!("Not enough elements in iterator")),
            }
        }

        Ok(result)
    }
}

impl<I: Iterator> IteratorExt for I {}

pub struct GridPoint {
    pub x: usize,
    pub y: usize,
}
