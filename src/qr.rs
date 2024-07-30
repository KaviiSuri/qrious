use std::iter::Enumerate;

use anyhow::{anyhow, Result};
use approx::relative_eq;
use image::GenericImageView;

use crate::{
    img::{self, ToVert},
    util::{IteratorExt, Rect},
    viz::Visualizer,
};

pub type MaskFn = fn(u32, u32) -> bool;

/// 010 -> black white black
pub fn get_mask_fn(mask: u8) -> Option<MaskFn> {
    match mask {
        0b000 => Some(|x: u32, _: u32| -> bool { x % 3 == 0 }),
        0b010 => Some(|x: u32, y: u32| -> bool { (x + y) % 2 == 0 }),
        _ => None,
    }
}

const FINDER_NUM_ELEMS: usize = 7;
const TIMER_PATTERN_OFFSET: usize = FINDER_NUM_ELEMS - 1;
const TIMER_PATTERN_START: usize = FINDER_NUM_ELEMS;
const FORMAT_PATTERN_OFFSET: usize = FINDER_NUM_ELEMS + 1;

fn find_elem_size(
    timing_x: u32,
    timing_iter_end: u32,
    get_pix_val: impl Fn(u32) -> bool,
    size: f32,
) -> f32 {
    let mut last_is_white = get_pix_val(timing_x);
    let mut num_elems = FINDER_NUM_ELEMS * 2 - 1;
    let mut timing_x = timing_x;

    while timing_x < timing_iter_end {
        let is_white = get_pix_val(timing_x);
        if is_white != last_is_white {
            num_elems += 1;
        }
        last_is_white = is_white;
        timing_x += 1;
    }

    return size / num_elems as f32;
}

/// check timing patterns, count number of alternating black/white
/// use it to estimate the size of the elements
fn find_elem_sizes(
    bounds: &Rect,
    img: &image::DynamicImage,
    finder_width: f32,
    finder_height: f32,
) -> (f32, f32) {
    let estimated_elem_width = finder_width / FINDER_NUM_ELEMS as f32;
    let estimated_elem_height = finder_height / FINDER_NUM_ELEMS as f32;

    // go to the middle of the timing pattern row
    let timing_row_center_px =
        (bounds.top() + (FINDER_NUM_ELEMS as f32 - 0.5) * estimated_elem_height) as u32;

    // go to the middle of the timing pattern column
    let timing_col_center_px =
        (bounds.left() + (FINDER_NUM_ELEMS as f32 - 0.5) * estimated_elem_width) as u32;

    // we go to inside the finder pattern
    let timing_iter_row_end = (bounds.right() - finder_width + estimated_elem_width / 2.0) as u32;
    let timing_iter_col_end =
        (bounds.bottom() - finder_height + estimated_elem_height / 2.0) as u32;

    let elem_width = find_elem_size(
        timing_col_center_px,
        timing_iter_row_end,
        |x| img::is_white(&img.get_pixel(x, timing_row_center_px)),
        bounds.width(),
    );
    let elem_height = find_elem_size(
        timing_row_center_px,
        timing_iter_col_end,
        |y| img::is_white(&img.get_pixel(timing_col_center_px, y)),
        bounds.height(),
    );

    return (elem_width, elem_height);
}

pub struct Code {
    pub bounds: Rect,
    elem_width: f32,
    elem_height: f32,
}

impl Code {
    pub fn new(img: &image::DynamicImage, mut visualizer: Option<&mut Visualizer>) -> Result<Self> {
        let finders = find_patterns(&img, visualizer.as_deref_mut())?;
        let mut finder_width = 0.0;
        let mut finder_height = 0.0;
        let mut top: f32 = f32::MAX;
        let mut bottom: f32 = 0.0;
        let mut left: f32 = f32::MAX;
        let mut right: f32 = 0.0;

        if finders.len() != 3 {
            return Err(anyhow!("Expected 3 finders, got {}", finders.len()));
        }

        for finder_rect in finders.iter() {
            if let Some(vis) = visualizer.as_deref_mut() {
                finder_rect.draw(vis, "yellow", None)?;
            }
            finder_width += finder_rect.width();
            finder_height += finder_rect.height();
            top = top.min(finder_rect.cy() - finder_rect.height() / 2.0);
            bottom = bottom.max(finder_rect.cy() + finder_rect.height() / 2.0);
            left = left.min(finder_rect.cx() - finder_rect.width() / 2.0);
            right = right.max(finder_rect.cx() + finder_rect.width() / 2.0);
        }
        finder_width /= finders.len() as f32;
        finder_height /= finders.len() as f32;

        let qr_rect = Rect::from_corners(left, top, right, bottom);

        let (elem_width, elem_height) = find_elem_sizes(&qr_rect, img, finder_width, finder_height);

        Ok(Self {
            bounds: qr_rect,
            elem_width,
            elem_height,
        })
    }

    #[allow(dead_code)]
    pub fn horiz_timing_iter(&self) -> HorizTimingIter {
        HorizTimingIter::new(self)
    }
    #[allow(dead_code)]
    pub fn vert_timing_iter(&self) -> VertTimingIter {
        VertTimingIter::new(self)
    }
    #[allow(dead_code)]
    pub fn horiz_format_iter(&self) -> HorizFormatIter {
        HorizFormatIter::new(self)
    }
    #[allow(dead_code)]
    pub fn vert_format_iter(&self) -> VertFormatIter {
        VertFormatIter::new(self)
    }
    #[allow(dead_code)]
    pub fn bit_iter<'a>(&'a self, img: &'a image::DynamicImage) -> Result<DataBitIter> {
        let mut mask_val = 0;
        for (i, module) in self
            .horiz_format_iter()
            .skip(2)
            .take_or_err(3)?
            .iter()
            .enumerate()
        {
            let bit = img::is_white_module(img, &module);
            if bit {
                mask_val |= 1 << (i);
            }
        }
        let mask_fn = get_mask_fn(mask_val).ok_or(anyhow!("No mask fn found {mask_val:#05b}"))?;

        Ok(DataBitIter::new(self, mask_fn, img))
    }

    #[allow(dead_code)]
    pub fn data_iter<'a>(&'a self, img: &'a image::DynamicImage) -> Result<DataByteIter> {
        DataByteIter::new(self.bit_iter(img)?)
    }

    #[allow(dead_code)]
    pub fn idx_to_module(&self, x: usize, y: usize) -> Rect {
        let left = (x as f32 * self.elem_width) + self.bounds.left();
        let top = (y as f32 * self.elem_height) + self.bounds.top();
        let right = left + self.elem_width;
        let bottom = top + self.elem_height;

        Rect::from_corners(left, top, right, bottom)
    }

    #[allow(dead_code)]
    pub fn num_horiz_elems(&self) -> usize {
        (self.bounds.width() / self.elem_width) as usize
    }

    #[allow(dead_code)]
    pub fn num_vert_elems(&self) -> usize {
        (self.bounds.height() / self.elem_height) as usize
    }
}

pub struct HorizTimingIter<'a> {
    code: &'a Code,
    x: usize,
}
impl<'a> HorizTimingIter<'a> {
    #[allow(dead_code)]
    fn new(code: &'a Code) -> Self {
        Self {
            code,
            x: TIMER_PATTERN_START,
        }
    }
}
impl Iterator for HorizTimingIter<'_> {
    type Item = Rect;

    fn next(&mut self) -> Option<Self::Item> {
        if self.x >= self.code.num_horiz_elems() - TIMER_PATTERN_START {
            return None;
        }
        let timing_rect = self.code.idx_to_module(self.x, TIMER_PATTERN_OFFSET);
        self.x += 1;
        Some(timing_rect)
    }
}

pub struct VertTimingIter<'a> {
    code: &'a Code,
    y: usize,
}
impl<'a> VertTimingIter<'a> {
    #[allow(dead_code)]
    fn new(code: &'a Code) -> Self {
        Self {
            code,
            y: TIMER_PATTERN_START,
        }
    }
}
impl Iterator for VertTimingIter<'_> {
    type Item = Rect;

    fn next(&mut self) -> Option<Self::Item> {
        if self.y >= self.code.num_vert_elems() - TIMER_PATTERN_START {
            return None;
        }
        let timing_rect = self.code.idx_to_module(TIMER_PATTERN_OFFSET, self.y);
        self.y += 1;
        Some(timing_rect)
    }
}

pub struct HorizFormatIter<'a> {
    code: &'a Code,
    x: usize,
}

impl<'a> HorizFormatIter<'a> {
    fn new(code: &'a Code) -> Self {
        Self { code, x: 0 }
    }
}
impl Iterator for HorizFormatIter<'_> {
    type Item = Rect;

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            if self.x >= self.code.num_horiz_elems() {
                return None;
            }

            let prev_x = self.x;

            self.x += 1;
            let second_half_start = self.code.num_horiz_elems() - FINDER_NUM_ELEMS - 1;

            if self.x >= FINDER_NUM_ELEMS + 1 && self.x < second_half_start {
                self.x = second_half_start;
            }

            if prev_x == TIMER_PATTERN_OFFSET {
                continue;
            }

            return Some(self.code.idx_to_module(prev_x, FORMAT_PATTERN_OFFSET));
        }
    }
}
pub struct VertFormatIter<'a> {
    code: &'a Code,
    y: usize,
}

impl<'a> VertFormatIter<'a> {
    fn new(code: &'a Code) -> Self {
        Self {
            code,
            y: code.num_vert_elems(),
        }
    }
}
impl Iterator for VertFormatIter<'_> {
    type Item = Rect;

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            if self.y == 0 {
                return None;
            }

            self.y -= 1;
            let first_half_end = self.code.num_vert_elems() - FINDER_NUM_ELEMS - 1;

            if self.y == first_half_end {
                self.y = FINDER_NUM_ELEMS + 1;
            }

            if self.y == TIMER_PATTERN_OFFSET {
                continue;
            }

            return Some(self.code.idx_to_module(FORMAT_PATTERN_OFFSET, self.y));
        }
    }
}

/// black is 1, white is 0 (different from masks)
pub struct DataBitIter<'a> {
    code: &'a Code,
    x: isize,
    y: isize,
    moving_vertically: bool,
    movement_direction: isize,
    mask_fn: MaskFn,
    img: &'a image::DynamicImage,
}

impl<'a> DataBitIter<'a> {
    fn new(code: &'a Code, mask_fn: MaskFn, img: &'a image::DynamicImage) -> Self {
        Self {
            code,
            x: code.num_horiz_elems() as isize - 2,
            y: code.num_vert_elems() as isize,
            moving_vertically: true,
            movement_direction: -1,
            mask_fn,
            img,
        }
    }

    fn should_turn_around(&self, x: isize, y: isize) -> bool {
        let finder_num_elems = FINDER_NUM_ELEMS as isize;
        let width = self.code.num_horiz_elems() as isize;
        let height = self.code.num_vert_elems() as isize;

        let x_left_of_left_finder = x < finder_num_elems + 2;
        let y_above_top_finder = y < finder_num_elems + 2;

        let in_tl_finder = x_left_of_left_finder && y_above_top_finder;

        if in_tl_finder {
            return true;
        }

        let y_below_bottom_finder = y >= height - finder_num_elems - 1;
        let in_bl_finder = x_left_of_left_finder && y_below_bottom_finder;

        if in_bl_finder {
            return true;
        }

        let x_right_of_right_finder = x > width - finder_num_elems - 1;
        let in_tr_finder = x_right_of_right_finder && y_above_top_finder;
        if in_tr_finder {
            return true;
        }

        let out_of_bounds = x >= width || x < 0 || y >= height || y < 0;
        if out_of_bounds {
            return true;
        }
        return false;
    }

    fn should_skip(&self, x: isize, y: isize) -> bool {
        return x == TIMER_PATTERN_OFFSET as isize || y == TIMER_PATTERN_OFFSET as isize;
    }

    fn do_zig_zag(&mut self) {
        if self.moving_vertically {
            self.x += 1;
            self.y += self.movement_direction as isize;
        } else {
            self.x -= 1;
        }

        self.moving_vertically = !self.moving_vertically;
    }

    fn do_turn_around(&mut self, last_move_vertical: bool) -> Result<()> {
        if !self.should_turn_around(self.x, self.y) {
            return Ok(());
        }

        if !last_move_vertical {
            eprintln!(
                "UNHANDLED HORIZONTAL OUT OF BOUNDS: ({}, {})",
                self.x, self.y
            );
            return Err(anyhow!("unimplemented"));
        }
        self.x -= 2;
        self.y -= self.movement_direction;
        self.movement_direction *= -1;
        self.moving_vertically = false;
        return Ok(());
    }
}
pub struct Output {
    #[allow(dead_code)]
    pub module: Rect,
    pub bit: bool,
    #[allow(dead_code)]
    pub x: isize,
    #[allow(dead_code)]
    pub y: isize,
}

impl Iterator for DataBitIter<'_> {
    type Item = Output;

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            let last_move_vertical = self.moving_vertically;
            self.do_zig_zag();

            if let Err(_) = self.do_turn_around(last_move_vertical) {
                return None;
            }

            if !self.should_skip(self.x, self.y) {
                break;
            }
        }
        let module = self.code.idx_to_module(self.x as usize, self.y as usize);
        let is_dark = !img::is_white_module(self.img, &module);
        return Some(Output {
            module,
            x: self.x,
            y: self.y,
            bit: is_dark != (self.mask_fn)(self.x as u32, self.y as u32),
        });
    }
}

pub struct DataByteIter<'a> {
    iter: Enumerate<DataBitIter<'a>>,
    #[allow(dead_code)]
    pub encoding: u8,
    #[allow(dead_code)]
    pub length: u8,
    num_bytes_read: u8,
}
const NUM_ENCODING_BITS: usize = 4;
const NUM_LENGTH_BITS: usize = 8;

impl<'a> DataByteIter<'a> {
    fn new(iter: DataBitIter<'a>) -> Result<Self> {
        let mut iter = iter.enumerate();
        let mut encoding: u8 = 0b0000;
        for (i, item) in iter.take_or_err(NUM_ENCODING_BITS)?.iter() {
            let bit = item.bit;
            if bit {
                encoding |= 1 << (NUM_ENCODING_BITS - 1 - i);
            }
        }

        let mut length: u8 = 0b0000;
        for (i, item) in iter.take_or_err(NUM_LENGTH_BITS)?.iter() {
            let bit = item.bit;
            if bit {
                length |= 1 << (NUM_LENGTH_BITS + NUM_ENCODING_BITS - 1 - i);
            }
        }

        Ok(Self {
            iter,
            encoding,
            length,
            num_bytes_read: 0,
        })
    }
}

impl Iterator for DataByteIter<'_> {
    type Item = u8;

    fn next(&mut self) -> Option<Self::Item> {
        if let Ok(i) = self.iter.take_or_err(8) {
            let mut next_byte: u8 = 0;
            for (i, output) in i.iter() {
                let bit_idx = (i - NUM_ENCODING_BITS - NUM_LENGTH_BITS) % 8;
                if output.bit {
                    next_byte |= 1 << (7 - bit_idx);
                }

                if bit_idx == 7 {
                    let ret = next_byte;
                    self.num_bytes_read += 1;
                    return Some(ret);
                }
            }
        }
        return None;
    }
}

#[derive(Debug, PartialEq)]
pub struct RleItem {
    start: u32,
    len: u32,
}

// 111000000111 -> [3, 6, 3]
fn run_length_encode<T: PartialEq>(iter: &mut impl Iterator<Item = T>) -> Vec<RleItem> {
    let mut result = Vec::new();
    let mut count = 1;
    let mut last = match iter.next() {
        Some(bit) => bit,
        None => return result,
    };

    let mut pos = 1;
    for bit in iter {
        if bit == last {
            count += 1;
        } else {
            result.push(RleItem {
                len: count,
                start: pos - count,
            });
            count = 1;
            last = bit;
        }
        pos += 1;
    }
    result.push(RleItem {
        len: count,
        start: pos - count,
    });
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_run_length_encode_blank() {
        let input: Vec<RleItem> = vec![];
        let expected = vec![];
        assert_eq!(run_length_encode(&mut input.into_iter()), expected);
    }

    #[test]
    fn test_run_length_encode_one_bit() {
        let input: Vec<u8> = vec![1];
        let expected: Vec<RleItem> = vec![RleItem { len: 1, start: 0 }];
        assert_eq!(run_length_encode(&mut input.into_iter()), expected);
    }

    #[test]
    fn test_run_length_encode_with_data() {
        let input: Vec<u8> = vec![1, 1, 1, 0, 0, 0, 0, 1, 1];
        let expected: Vec<RleItem> = vec![
            RleItem { start: 0, len: 3 },
            RleItem { start: 3, len: 4 },
            RleItem { start: 7, len: 2 },
        ];
        assert_eq!(run_length_encode(&mut input.into_iter()), expected);
    }
}

struct FinderCandidate1D {
    center: f32,
    length: f32,
}

fn is_almost_same(a: u32, b: u32) -> bool {
    let af = a as f32;
    let bf = b as f32;

    (af / bf - 1.0).abs() < 0.2
}

fn find_candidates(rle: &[RleItem]) -> Vec<FinderCandidate1D> {
    let mut res = vec![];
    if rle.len() < 5 {
        return res;
    }

    let end = rle.len() - 5;

    for i in 0..end {
        // TODO: Allow for errors
        let reference = rle[i].len;
        if !is_almost_same(rle[i + 1].len, reference) {
            continue;
        }
        if !is_almost_same(rle[i + 2].len, reference * 3) {
            continue;
        }
        if !is_almost_same(rle[i + 3].len, reference) {
            continue;
        }
        if !is_almost_same(rle[i + 4].len, reference) {
            continue;
        }
        let length = (rle[i + 4].start + rle[i + 4].len - rle[i].start) as f32;

        res.push(FinderCandidate1D {
            center: rle[i + 2].start as f32 + (rle[i + 2].len as f32) / 2.0,
            length,
        })
    }

    res
}

fn add_rect_to_bucket(buckets: &mut Vec<Vec<Rect>>, rect: Rect) {
    for bucket in buckets.iter_mut() {
        for bucket_rect in bucket.iter_mut() {
            if relative_eq!(bucket_rect.cx(), rect.cx(), epsilon = 10.0)
                && relative_eq!(bucket_rect.cy(), rect.cy(), epsilon = 10.0)
            {
                bucket.push(rect);
                return;
            }
        }
    }
    buckets.push(Vec::new());
    buckets
        .last_mut()
        .expect("We just pushed a bucket")
        .push(rect);
}

/// Finds the finder patterns using the pattern 1:1:3:1:1
/// It does it horizontally and vertically, and finds intersection between them
/// to cluster and find 3 points at the end.
pub fn find_patterns(
    img: &image::DynamicImage,
    mut visualizer: Option<&mut Visualizer>,
) -> Result<Vec<Rect>> {
    let (width, height) = img.dimensions();

    use crate::img::ToHoriz;
    let mut horizontal_candidates: Vec<Rect> = Vec::new();

    for x in 0..width {
        let encoding = run_length_encode(&mut img.to_vert(x));
        // print!("encoding = [");
        // for rle in encoding.iter() {
        //     print!("{:?}, ", rle.len);
        // }
        // println!("]");
        let candidates = find_candidates(&encoding);
        for FinderCandidate1D { center, length } in candidates {
            let cx = x as f32 + 0.5;
            horizontal_candidates.push(Rect::from_center_and_size(cx, center, length, 0.0));
            if let Some(ref mut vis) = visualizer {
                vis.draw_circle(cx, center, 0.5, "blue")?;
            }
        }
    }

    let mut vertical_candidates: Vec<Rect> = Vec::new();

    for y in 0..height {
        let encoding = run_length_encode(&mut img.to_horiz(y));
        let candidates = find_candidates(&encoding);
        for FinderCandidate1D { center, length } in candidates {
            let cy = y as f32 + 0.5;
            vertical_candidates.push(Rect::from_center_and_size(center, cy, 0.0, length));
            if let Some(ref mut vis) = visualizer {
                vis.draw_circle(center, cy, 0.5, "red")?;
            }
        }
    }

    let mut buckets: Vec<Vec<Rect>> = Vec::new();

    for v in vertical_candidates.iter() {
        for h in horizontal_candidates.iter() {
            if relative_eq!(v.cx(), h.cx(), epsilon = 1.0)
                && relative_eq!(v.cy(), h.cy(), epsilon = 1.0)
            {
                let cx = (v.cx() + h.cx()) / 2.0;
                let cy = (v.cy() + h.cy()) / 2.0;
                let width = h.width();
                let height = v.height();
                let combined = Rect::from_center_and_size(cx, cy, width, height);
                add_rect_to_bucket(&mut buckets, combined);
            }
        }
    }

    let finders = buckets
        .iter()
        .map(|bucket| {
            let mut cx = 0.0;
            let mut cy = 0.0;
            let mut width = 0.0;
            let mut height = 0.0;
            for rect in bucket.iter() {
                cx += rect.cx();
                cy += rect.cy();
                width += rect.width();
                height += rect.height();
            }
            let len = bucket.len() as f32;
            Rect::from_center_and_size(cx / len, cy / len, width / len, height / len)
        })
        .collect::<Vec<Rect>>();
    Ok(finders)
}
