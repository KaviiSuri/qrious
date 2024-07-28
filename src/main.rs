mod img;
mod qr;
mod util;
mod viz;
use anyhow::Result;
use image::{GenericImageView, ImageReader};
use std::env;

use crate::viz::Visualizer;

fn main() -> Result<()> {
    let args = env::args().collect::<Vec<String>>();
    let file_name = &args[1];
    let img = ImageReader::open(file_name)?;
    let img = img.decode()?.grayscale();

    let (width, height) = img.dimensions();
    let mut visualizer = Visualizer::new(width, height, file_name.into())?;

    let code = qr::Code::new(&img)?;

    for module in code.horiz_timing_iter() {
        module.draw(&mut visualizer, "red")?;
    }
    for module in code.horiz_format_iter() {
        module.draw(&mut visualizer, "purple")?;
    }

    for (idx, module) in code.data_iter().take(200).enumerate() {
        module.draw(&mut visualizer, "orange")?;
        visualizer.draw_text(module.cx(), module.cy(), idx.to_string().as_str(), "red")?;
    }

    visualizer.finish()?;
    return Ok(());
}
