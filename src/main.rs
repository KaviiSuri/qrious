mod img;
mod qr;
mod util;
mod viz;
use anyhow::Result;
use image::{GenericImageView, ImageReader};
use qr::{HorizFormatIter, Output};
use std::env;

use crate::{qr::get_mask_fn, viz::Visualizer};

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
    let args = env::args().collect::<Vec<String>>();
    let file_name = &args[1];
    let img = ImageReader::open(file_name)?;
    let img = img.decode()?.grayscale();

    let (width, height) = img.dimensions();
    let mut visualizer = Visualizer::new(width, height, file_name.into())?;

    let code = qr::Code::new(&img, Some(&mut visualizer))?;

    for module in code.horiz_timing_iter() {
        module.draw(&mut visualizer, "red")?;
    }

    let mask_fn = inspect_format_iter(&img, code.horiz_format_iter(), &mut visualizer)?;

    for (idx, Output { module, x, y }) in code.data_iter().take(200).enumerate() {
        let is_white = img::is_white_module(&img, &module);
        module.draw(&mut visualizer, "orange")?;
        visualizer.draw_text(module.cx(), module.cy(), idx.to_string().as_str(), "red")?;
    }

    visualizer.finish()?;
    return Ok(());
}
