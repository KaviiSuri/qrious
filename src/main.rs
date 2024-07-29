mod img;
mod qr;
mod util;
mod viz;
use anyhow::Result;
use clap::Parser;
use image::{GenericImageView, ImageReader};
use qr::{HorizFormatIter, HorizTimingIter, Output};
use std::{fs, path::PathBuf};

use crate::{qr::get_mask_fn, viz::Visualizer};

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Cli {
    /// The input QR code image
    input: PathBuf,

    /// The output directory
    output: PathBuf,
}

fn inspect_timing_iter(iter: HorizTimingIter, visualizer: &mut Visualizer) -> Result<()> {
    for module in iter {
        module.draw(visualizer, "red")?;
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
        module.draw(visualizer, "purple")?;
        let bit = img::is_white_module(&img, &module);
        if (2..5).contains(&idx) && bit {
            mask_val |= 1 << (idx - 2);
        }
    }

    let mask_fn = get_mask_fn(mask_val).unwrap();

    return Ok(mask_fn);
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

    for (idx, Output { module, x, y }) in code.data_iter().take(200).enumerate() {
        let is_dark = !img::is_white_module(&img, &module);
        if is_dark == mask_fn(x as u32, y as u32) {
            module.draw(&mut decoded_vis, "black")?;
        }
        module.draw(&mut dbg_vis, "orange")?;
        dbg_vis.draw_text(module.cx(), module.cy(), idx.to_string().as_str(), "red")?;
    }

    dbg_vis.finish()?;
    return Ok(());
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
