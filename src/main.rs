mod img;
mod qr;
mod util;
mod viz;
use anyhow::{anyhow, Result};
use clap::Parser;
use image::{GenericImageView, ImageReader};
use qr::{HorizFormatIter, HorizTimingIter, Output};
use std::{fs, path::PathBuf};

use crate::{qr::get_mask_fn, viz::Visualizer};

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

const NUM_ENCODING_BITS: usize = 4;
const NUM_LENGTH_BITS: usize = 8;

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Cli {
    /// The input QR code image
    input: PathBuf,

    /// The output directory
    output: PathBuf,
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    let file_name = init_output_dir(&cli)?;

    let img = ImageReader::open(&cli.output.join(&file_name))?;
    let img = img.decode()?.grayscale();
    let (width, height) = img.dimensions();

    let mut dbg_vis = Visualizer::new(
        width,
        height,
        &cli.output.join("debug.svg"),
        Some(file_name),
    )?;
    let mut decoded_vis = Visualizer::new(width, height, &cli.output.join("decoded.svg"), None)?;

    let code = qr::Code::new(&img, Some(&mut dbg_vis))?;

    inspect_timing_iter(code.horiz_timing_iter(), &mut dbg_vis)?;
    let mask_fn = inspect_format_iter(&img, code.horiz_format_iter(), &mut dbg_vis)?;

    code.bounds.draw(&mut decoded_vis, "gray", None)?;

    let mut iter = code.data_iter().enumerate();

    let mut encoding: u8 = 0b0000;
    for (i, item) in iter.take_or_err(NUM_ENCODING_BITS)?.iter() {
        let bit = handle_data(item, *i, &img, &mask_fn, &mut decoded_vis, &mut dbg_vis)?;
        if bit {
            encoding |= 1 << (3 - i);
        }
    }
    println!("encoding = {encoding:#06b}");

    let mut length: u8 = 0;
    for (i, item) in iter.take_or_err(NUM_LENGTH_BITS)?.iter() {
        let bit = handle_data(item, *i, &img, &mask_fn, &mut decoded_vis, &mut dbg_vis)?;
        if bit {
            length |= 1 << (11 - i);
        }
    }
    println!("length = {length}");

    let mut data: Vec<u8> = Vec::with_capacity(length as usize);
    let mut next_byte: u8 = 0;
    for (i, item) in iter.take_or_err((length as usize) * 8)?.iter() {
        let bit = handle_data(item, *i, &img, &mask_fn, &mut decoded_vis, &mut dbg_vis)?;

        let bit_idx = (i - NUM_ENCODING_BITS - NUM_LENGTH_BITS) % 8;

        if bit {
            next_byte |= 1 << (7 - bit_idx);
        }

        if bit_idx == 7 {
            data.push(next_byte);
            next_byte = 0;
        }
    }
    println!("data = {:?}", data);
    // print ascii
    let ascii: String = data.iter().map(|&b| b as char).collect();
    println!("ascii = {:?}", ascii);

    return Ok(());
}

fn handle_data(
    item: &Output,
    idx: usize,
    img: &image::DynamicImage,
    mask_fn: &impl Fn(u32, u32) -> bool,
    decoded_vis: &mut Visualizer,
    dbg_vis: &mut Visualizer,
) -> Result<bool> {
    let Output { module, x, y } = item;
    let is_dark = !img::is_white_module(img, &module);
    let bit = is_dark != mask_fn(*x as u32, *y as u32);
    if bit {
        module.draw(decoded_vis, "black", Some("black"))?;
    }
    module.draw(dbg_vis, "orange", None)?;
    dbg_vis.draw_text(module.cx(), module.cy(), idx.to_string().as_str(), "red")?;

    Ok(bit)
}

fn init_output_dir(cli: &Cli) -> Result<PathBuf> {
    fs::create_dir_all(&cli.output)?;
    // copy the input file to the output directory
    let extension = cli.input.extension().unwrap();
    let file_name = PathBuf::new()
        .with_file_name("QR")
        .with_extension(extension);
    let output_file_name = cli.output.join(file_name.clone());
    fs::copy(&cli.input, &output_file_name)?;

    Ok(file_name)
}

fn inspect_timing_iter(iter: HorizTimingIter, visualizer: &mut Visualizer) -> Result<()> {
    for module in iter {
        module.draw(visualizer, "red", None)?;
    }
    Ok(())
}

fn inspect_format_iter(
    img: &image::DynamicImage,
    iter: HorizFormatIter,
    visualizer: &mut Visualizer,
) -> Result<impl Fn(u32, u32) -> bool> {
    let mut mask_val: u8 = 0;
    for (idx, module) in iter.enumerate() {
        module.draw(visualizer, "purple", None)?;
        let bit = img::is_white_module(&img, &module);
        if (2..5).contains(&idx) && bit {
            mask_val |= 1 << (idx - 2);
        }
    }

    let mask_fn = get_mask_fn(mask_val).unwrap();

    return Ok(mask_fn);
}
