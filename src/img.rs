use anyhow::anyhow;
use anyhow::Result;
use image::{GenericImageView, Pixel};

use crate::util::Rect;

pub fn is_white<T: Pixel<Subpixel = u8>>(pixel: &T) -> bool {
    pixel.to_luma().0[0] > 128
}

pub fn is_white_module(img: &image::DynamicImage, module: &Rect) -> bool {
    let (left, top, right, bottom) = module.to_corners();

    let start_y = top.ceil() as u32;
    let end_y = bottom.floor() as u32;
    let start_x = left.ceil() as u32;
    let end_x = right.floor() as u32;

    let mut val = 0;
    for y in start_y..end_y {
        for x in start_x..end_x {
            if is_white(&img.get_pixel(x, y)) {
                val += 1;
            }
        }
    }

    val as f32 / (module.width() * module.height()) > 0.5
}

pub struct HorizIterator<'a> {
    img: &'a image::DynamicImage,
    x: u32,
    y: u32,
}

impl<'a> HorizIterator<'a> {
    pub fn new(img: &'a image::DynamicImage, y: u32) -> Result<Self> {
        if y >= img.height() {
            Err(anyhow!("invalid y for the image"))
        } else {
            Ok(Self { img, x: 0, y })
        }
    }
}

impl<'a> Iterator for HorizIterator<'a> {
    type Item = bool;

    fn next(&mut self) -> Option<Self::Item> {
        let result = is_white(&self.img.get_pixel(self.x, self.y));
        self.x += 1;
        if self.x >= self.img.width() {
            None
        } else {
            Some(result)
        }
    }
}

pub trait ToHoriz {
    fn to_horiz(&self, y: u32) -> HorizIterator;
}

impl ToHoriz for image::DynamicImage {
    fn to_horiz(&self, y: u32) -> HorizIterator {
        match HorizIterator::new(self, y) {
            Ok(iter) => iter,
            Err(_) => HorizIterator {
                img: self,
                x: self.width() - 1,
                y,
            },
        }
    }
}

pub struct VertIterator<'a> {
    img: &'a image::DynamicImage,
    x: u32,
    y: u32,
}

impl<'a> VertIterator<'a> {
    pub fn new(img: &'a image::DynamicImage, x: u32) -> Result<Self> {
        if x >= img.width() {
            Err(anyhow!("invalid y for the image"))
        } else {
            Ok(Self { img, x, y: 0 })
        }
    }
}

impl<'a> Iterator for VertIterator<'a> {
    type Item = bool;

    fn next(&mut self) -> Option<Self::Item> {
        let result = is_white(&self.img.get_pixel(self.x, self.y));
        self.y += 1;
        if self.y >= self.img.height() {
            None
        } else {
            Some(result)
        }
    }
}

pub trait ToVert {
    fn to_vert(&self, x: u32) -> VertIterator;
}

impl ToVert for image::DynamicImage {
    fn to_vert(&self, x: u32) -> VertIterator {
        match VertIterator::new(self, x) {
            Ok(iter) => iter,
            Err(_) => VertIterator {
                img: self,
                x,
                y: self.height() - 1,
            },
        }
    }
}
