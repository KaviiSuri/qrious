use anyhow::Result;
use approx::relative_eq;
use image::GenericImageView;

use crate::{img::ToVert, util::Rect};

pub fn get_mask_fn(mask: u8) -> Option<impl Fn(u32, u32) -> bool> {
    match mask {
        0b000 => Some(|x: u32, _: u32| -> bool { x % 3 == 0 }),
        _ => None,
    }
}

pub struct Code {
    bounds: Rect,
    elem_width: f32,
    elem_height: f32,
}

impl Code {
    pub fn new(img: &image::DynamicImage) -> Result<Self> {
        let finders = find_patterns(&img)?;
        let mut elem_width = 0.0;
        let mut elem_height = 0.0;
        let mut top: f32 = f32::MAX;
        let mut bottom: f32 = 0.0;
        let mut left: f32 = f32::MAX;
        let mut right: f32 = 0.0;

        for finder_rect in finders.iter() {
            elem_width += finder_rect.width() / 7.0;
            elem_height += finder_rect.height() / 7.0;
            top = top.min(finder_rect.cy() - finder_rect.height() / 2.0);
            bottom = bottom.max(finder_rect.cy() + finder_rect.height() / 2.0);
            left = left.min(finder_rect.cx() - finder_rect.width() / 2.0);
            right = right.max(finder_rect.cx() + finder_rect.width() / 2.0);
        }
        elem_width /= finders.len() as f32;
        elem_height /= finders.len() as f32;

        let qr_rect = Rect::from_corners(left, top, right, bottom);

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
    pub fn horiz_format_iter(&self) -> HorizFormatIter {
        HorizFormatIter::new(self)
    }
    #[allow(dead_code)]
    pub fn data_iter(&self) -> DataIter {
        DataIter::new(self)
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
    x: f32,
}
impl<'a> HorizTimingIter<'a> {
    #[allow(dead_code)]
    fn new(code: &'a Code) -> Self {
        Self {
            code,
            x: code.bounds.left() + 7.5 * code.elem_width,
        }
    }
}
impl Iterator for HorizTimingIter<'_> {
    type Item = Rect;

    fn next(&mut self) -> Option<Self::Item> {
        if self.x >= self.code.bounds.right() - 6.5 * self.code.elem_width {
            return None;
        }
        let timing_rect = Rect::from_center_and_size(
            self.x,
            self.code.bounds.top() + 6.5 * self.code.elem_height,
            self.code.elem_width,
            self.code.elem_height,
        );
        self.x += self.code.elem_width;
        Some(timing_rect)
    }
}

const FINDER_NUM_ELEMS: usize = 7;
const TIMER_PATTERN_OFFSET: usize = 6;

pub struct HorizFormatIter<'a> {
    code: &'a Code,
    x: f32,
    first_half_end: f32,
    second_half_start: f32,
    top: f32,
}

impl<'a> HorizFormatIter<'a> {
    fn new(code: &'a Code) -> Self {
        Self {
            code,
            x: code.bounds.left(),
            first_half_end: code.bounds.left() + code.elem_width * FINDER_NUM_ELEMS as f32,
            second_half_start: code.bounds.right() - (FINDER_NUM_ELEMS as f32 * code.elem_width),
            top: code.bounds.top() + (FINDER_NUM_ELEMS + 1) as f32 * code.elem_height,
        }
    }
}
impl Iterator for HorizFormatIter<'_> {
    type Item = Rect;

    fn next(&mut self) -> Option<Self::Item> {
        if self.x >= (self.code.bounds.right() - self.code.elem_width / 2.0) {
            return None;
        }

        let res = Rect::from_corners(
            self.x,
            self.top,
            self.x + self.code.elem_width,
            self.top + self.code.elem_height,
        );
        self.x += self.code.elem_width;

        if self.x >= (self.first_half_end - self.code.elem_width / 2.0)
            && self.x <= (self.second_half_start - self.code.elem_width / 2.0)
        {
            self.x = self.second_half_start;
        }

        Some(res)
    }
}

pub struct DataIter<'a> {
    code: &'a Code,
    x: isize,
    y: isize,
    next_move_vertical: bool,
    moving_up: bool,
}

impl<'a> DataIter<'a> {
    fn new(code: &'a Code) -> Self {
        println!("Code bounds: {:?}", code.bounds);
        Self {
            code,
            x: code.num_horiz_elems() as isize - 2,
            y: code.num_vert_elems() as isize,
            next_move_vertical: true,
            moving_up: true,
        }
    }

    fn should_turn_around(&self, x: isize, y: isize) -> bool {
        let finder_num_elems = FINDER_NUM_ELEMS as isize;
        let width = self.code.num_horiz_elems() as isize;
        let height = self.code.num_vert_elems() as isize;
        let x_in_finder_region = x < finder_num_elems + 2
            || (x > width - finder_num_elems - 1 && y < finder_num_elems + 2);

        let y_in_finder_region = y < finder_num_elems + 2
            || (y >= height - finder_num_elems - 1 && x < finder_num_elems + 2);

        dbg!(x_in_finder_region, y_in_finder_region);

        // TODO: Move Module calculations into abstraction with floating point handling
        (x_in_finder_region && y_in_finder_region)
            || (x >= width)
            || (y >= height)
            || (x < 0)
            || (y < 0)
    }

    fn should_skip(&self, x: isize, y: isize) -> bool {
        return x == TIMER_PATTERN_OFFSET as isize || y == TIMER_PATTERN_OFFSET as isize;
    }

    fn inc_pos(&mut self) {
        if self.next_move_vertical {
            self.x += 1;
            let y_adjust = if self.moving_up { -1 } else { 1 };
            self.y += y_adjust;
            while self.should_skip(self.x, self.y) {
                println!("Skipping {} {}", self.x, self.y);
                self.y += y_adjust;
            }
        } else {
            self.x -= 1;
        }
    }
}

impl Iterator for DataIter<'_> {
    type Item = Rect;

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            let old_y = self.y;
            let old_x = self.x;

            self.inc_pos();

            if self.should_turn_around(self.x, self.y) {
                if self.next_move_vertical {
                    self.y = old_y;
                    self.x = old_x;
                    self.x -= 1;
                    self.moving_up = !self.moving_up;
                    self.next_move_vertical = false;
                    return Some(self.code.idx_to_module(self.x as usize, self.y as usize));
                } else {
                    eprintln!("UNHANDLED HORIZONTAL OUT OF BOUNDS");
                    return None;
                }
            }
            self.next_move_vertical = !self.next_move_vertical;
            return Some(self.code.idx_to_module(self.x as usize, self.y as usize));
        }
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

fn find_candidates(rle: &[RleItem]) -> Vec<FinderCandidate1D> {
    let mut res = vec![];
    if rle.len() < 5 {
        return res;
    }

    let end = rle.len() - 5;

    for i in 0..end {
        // TODO: Allow for errors
        let reference = rle[i].len;
        if rle[i + 1].len != reference {
            continue;
        }
        if rle[i + 2].len / reference != 3 {
            continue;
        }
        if rle[i + 3].len != reference {
            continue;
        }
        if rle[i + 4].len != reference {
            continue;
        }

        res.push(FinderCandidate1D {
            center: rle[i + 2].start as f32 + (rle[i + 2].len as f32) / 2.0,
            length: (reference as f32) * 7.0,
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

pub fn find_patterns(img: &image::DynamicImage) -> Result<Vec<Rect>> {
    let (width, height) = img.dimensions();

    use crate::img::ToHoriz;
    let mut horizontal_candidates: Vec<Rect> = Vec::new();

    for x in 0..width {
        let encoding = run_length_encode(&mut img.to_vert(x));
        let candidates = find_candidates(&encoding);
        for FinderCandidate1D { center, length } in candidates {
            let cx = x as f32 + 0.5;
            horizontal_candidates.push(Rect::from_center_and_size(cx, center, length, 0.0));
            // visualizer.draw_circle(cx, center, 0.5, "blue")?;
        }
    }

    let mut vertical_candidates: Vec<Rect> = Vec::new();

    for y in 0..height {
        let encoding = run_length_encode(&mut img.to_horiz(y));
        let candidates = find_candidates(&encoding);
        for FinderCandidate1D { center, length } in candidates {
            let cy = y as f32 + 0.5;
            vertical_candidates.push(Rect::from_center_and_size(center, cy, 0.0, length));
            // visualizer.draw_circle(center, cy, 0.5, "red")?;
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
