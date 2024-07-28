use std::{
    fs::{self, File},
    path::PathBuf,
};

use anyhow::anyhow;
use anyhow::Result;
use xml::{writer::XmlEvent, EmitterConfig, EventWriter};

const DEBUG_OUTPUT: &str = "debug.svg";

pub struct Visualizer {
    #[allow(dead_code)]
    width: u32,
    #[allow(dead_code)]
    height: u32,
    #[allow(dead_code)]
    img_path: PathBuf,
    svg_writer: EventWriter<File>,

    stroke_width: f32,
    font_size: f32,
}

impl Visualizer {
    pub fn new(width: u32, height: u32, img_path: PathBuf) -> Result<Self> {
        let file = fs::File::create(DEBUG_OUTPUT).unwrap();
        let mut svg_writer = EmitterConfig::new()
            .perform_indent(true)
            .create_writer(file);

        svg_writer.write(
            XmlEvent::start_element("svg")
                .attr("xmlns", "http://www.w3.org/2000/svg")
                .attr("width", &width.to_string())
                .attr("height", &height.to_string())
                .attr("style", "zoom: 2"),
        )?;

        svg_writer.write(
            XmlEvent::start_element("image")
                .attr("href", &img_path.to_str().ok_or(anyhow!("Invalid path"))?)
                .attr("width", &width.to_string())
                .attr("height", &height.to_string()),
        )?;

        svg_writer.write(XmlEvent::end_element())?;

        Ok(Visualizer {
            width,
            height,
            img_path,
            svg_writer,
            stroke_width: (width as f32) / 1000.0,
            font_size: (width as f32) / 50.0,
        })
    }

    #[allow(dead_code)]
    pub fn draw_circle(&mut self, x: f32, y: f32, r: f32, color: &str) -> Result<()> {
        self.svg_writer.write(
            XmlEvent::start_element("circle")
                .attr("cx", &x.to_string())
                .attr("cy", &y.to_string())
                .attr("r", &r.to_string())
                .attr("fill", color),
        )?;
        self.svg_writer.write(XmlEvent::end_element())?;
        Ok(())
    }

    #[allow(dead_code)]
    pub fn draw_rect(&mut self, cx: f32, cy: f32, w: f32, h: f32, color: &str) -> Result<()> {
        let x = cx - w / 2.0;
        let y = cy - h / 2.0;
        self.svg_writer.write(
            XmlEvent::start_element("rect")
                .attr("x", &x.to_string())
                .attr("y", &y.to_string())
                .attr("width", &w.to_string())
                .attr("height", &h.to_string())
                .attr("stroke", color)
                .attr("fill", "none")
                .attr("stroke-width", "0.5"),
        )?;
        self.svg_writer.write(XmlEvent::end_element())?;
        Ok(())
    }

    #[allow(dead_code)]
    pub fn draw_text(&mut self, x: f32, y: f32, text: &str, color: &str) -> Result<()> {
        let x = x - 0.5;
        let y = y + 0.5;
        self.svg_writer.write(
            XmlEvent::start_element("text")
                .attr("x", &x.to_string())
                .attr("y", &y.to_string())
                .attr("font-size", self.font_size.to_string().as_str())
                .attr("stroke-width", self.stroke_width.to_string().as_str())
                .attr("fill", color),
        )?;
        self.svg_writer.write(XmlEvent::characters(text))?;
        self.svg_writer.write(XmlEvent::end_element())?;
        Ok(())
    }

    pub fn finish(&mut self) -> Result<()> {
        self.svg_writer.write(XmlEvent::end_element())?;
        Ok(())
    }
}
