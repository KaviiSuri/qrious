use anyhow::Result;
use base64::Engine;
use image::{DynamicImage, GenericImageView, ImageReader, Pixel};
use std::{env, io::Cursor};

fn main() -> Result<()> {
    let args = env::args().collect::<Vec<String>>();
    let file_name = &args[1];
    let img = ImageReader::open(file_name)?;
    let img = img.decode()?.grayscale();
    let (width, height) = img.dimensions();

    // println!("Image dimensions: {} x {}", width, height);

    // for y in 0..height {
    //     for x in 0..width {
    //         let pixel = img.get_pixel(x, y).to_luma();
    //         if pixel[0] > 128 {
    //             print!("@");
    //         } else {
    //             print!(" ");
    //         }
    //     }
    //     println!();
    // }

    let b64 = image_to_b64(&img)?;
    let svg_content = format!(
        r#"<svg xmlns="http://www.w3.org/2000/svg" width="{width}" height="{height}">
            <image href="data:image/png;base64,{b64}" width="{width}" height="{height}" />
        </svg>"#,
        width = width,
        height = height,
        b64 = b64
    );

    println!("{}", svg_content);

    return Ok(());
}

fn image_to_b64(img: &DynamicImage) -> Result<String> {
    use base64::prelude::BASE64_STANDARD;
    let mut buf = Cursor::new(Vec::new());
    img.write_to(&mut buf, image::ImageFormat::Png)?;
    let image_bytes = buf.into_inner();
    let b64 = BASE64_STANDARD.encode(&image_bytes);
    return Ok(b64);
}
