mod img;
mod qr;
mod util;
mod viz;
use anyhow::{anyhow, Result};
use clap::Parser;
use image::{GenericImageView, ImageReader};
use qr::{DataBitIter, HorizFormatIter, HorizTimingIter, Output, VertFormatIter, VertTimingIter};
use std::{fs, path::PathBuf};
use util::Rect;

use crate::{qr::AlignmentPatternIter, viz::Visualizer};

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
    code.bounds.draw(&mut dbg_vis, "gray", None)?;
    code.bounds.draw(&mut decoded_vis, "gray", None)?;
    inspect_timing(code.horiz_timing_iter(), &img)?;
    inspect_timing(code.vert_timing_iter(), &img)?;

    let alignment_iter = AlignmentPatternIter::for_code(&code, &img);
    for pattern in alignment_iter {
        pattern
            .module
            .draw(&mut dbg_vis, "blue", Some("rgba(0, 255, 0, 0.6)"))?;
    }

    viz_timing_iter(
        code.horiz_timing_iter(),
        code.vert_timing_iter(),
        &mut dbg_vis,
    )?;
    viz_format_iter(
        code.horiz_format_iter(),
        code.vert_format_iter(),
        &mut dbg_vis,
    )?;
    // viz_bits(code.bit_iter(&img)?, &mut decoded_vis, &mut dbg_vis)?;

    let iter = code.data_iter(&img)?;
    let encoding = iter.encoding;
    println!("encoding = {encoding:#05b}");
    let data: Vec<_> = iter.collect();
    println!("data = {:?}", data);
    if encoding == 0b0010 || encoding == 0b0100 {
        let ascii: String = data.iter().map(|&b| b as char).collect();
        println!("ascii = {:?}", ascii);
    } else {
        return Err(anyhow!("Unsupported encoding {encoding:#05b}"));
    }

    return Ok(());
}

fn viz_bits(
    iter: DataBitIter,
    decoded_vis: &mut Visualizer,
    dbg_vis: &mut Visualizer,
) -> Result<()> {
    for (i, item) in iter.enumerate() {
        let Output { module, bit, .. } = item;
        if bit {
            module.draw(decoded_vis, "black", Some("black"))?;
        }
        module.draw(dbg_vis, "orange", None)?;
        dbg_vis.draw_text(module.cx(), module.cy(), i.to_string().as_str(), "red")?;
    }
    Ok(())
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

fn viz_timing_iter(
    horiz_iter: HorizTimingIter,
    vert_iter: VertTimingIter,
    visualizer: &mut Visualizer,
) -> Result<()> {
    for module in horiz_iter {
        module.draw(visualizer, "red", None)?;
    }
    for module in vert_iter {
        module.draw(visualizer, "red", None)?;
    }
    Ok(())
}

fn viz_format_iter(
    horiz_iter: HorizFormatIter,
    vert_iter: VertFormatIter,
    visualizer: &mut Visualizer,
) -> Result<()> {
    for module in horiz_iter {
        module.draw(visualizer, "purple", None)?;
    }
    for module in vert_iter {
        module.draw(visualizer, "purple", None)?;
    }
    Ok(())
}

fn inspect_timing(iter: impl Iterator<Item = Rect>, img: &image::DynamicImage) -> Result<()> {
    let mut expected = true;
    for module in iter {
        if img::is_white_module(img, &module) != expected {
            return Err(anyhow!("Timing module {:?} is not as expected", &module));
        }
        expected = !expected;
    }
    Ok(())
}
