//! Control and draw to the Inky display

use crate::{
    eeprom::{DisplayVariant, EEPROM},
    hardware::{
        display::{InkyDisplay, RefreshSpeed},
        inkye673::InkyE673,
        inkywhat::InkyWhat,
        inkyphatssd1608::InkyPhatSsd1608,
    },
    core::colors::Color,
};

use anyhow::{Error, Result, bail, ensure};

pub trait Drawable {
    fn coordinates(&self) -> Vec<(usize, usize)>;
}

pub struct Line {
    start: (isize, isize),
    end: (isize, isize),
}

impl Line {
    pub fn new(start: (isize, isize), end: (isize, isize)) -> Self {
        Self { start, end }
    }

    // Returns a vector of coordinates along the line using Bresenham's algorithm
    fn line_coordinates(&self) -> Vec<(usize, usize)> {
        let mut result = Vec::new();

        let (mut x0, mut y0) = self.start;
        let (x1, y1) = self.end;

        let dx = x1 - x0;
        let dy = -(y1 - y0);

        let sx = if x0 < x1 { 1 } else { -1 };
        let sy = if y0 < y1 { 1 } else { -1 };

        let mut err = dx + dy;

        loop {
            result.push((x0 as usize, y0 as usize));
            if x0 == x1 && y0 == y1 {
                break;
            }
            let e2 = 2 * err;
            if e2 >= dy {
                err += dy;
                x0 += sx;
            }
            if e2 <= dx {
                err += dx;
                y0 += sy;
            }
        }

        result
    }
}

impl Drawable for Line {
    fn coordinates(&self) -> Vec<(usize, usize)> {
        self.line_coordinates()
    }
}

pub struct Rectangle {
    top_left: (usize, usize),
    bottom_right: (usize, usize),
}

impl Rectangle {
    pub fn new(top_left: (usize, usize), bottom_right: (usize, usize)) -> Self {
        Self {
            top_left,
            bottom_right,
        }
    }

    // Returns a vector of coordinates inside the rectangle
    fn rectangle_coordinates(&self) -> Vec<(usize, usize)> {
        let mut result = Vec::new();

        for row in self.top_left.0..=self.bottom_right.0 {
            for col in self.top_left.1..=self.bottom_right.1 {
                result.push((row, col));
            }
        }

        result
    }
}

impl Drawable for Rectangle {
    fn coordinates(&self) -> Vec<(usize, usize)> {
        self.rectangle_coordinates()
    }
}

pub struct Canvas {
    width: usize,
    height: usize,
    pixels: Vec<Vec<Color>>,
}

impl Canvas {
    /// Create a new drawing canvas with a width and height
    fn new(width: usize, height: usize) -> Canvas {
        Canvas {
            width,
            height,
            pixels: vec![vec![Color::White; width ]; height],
        } 
    }

    /// Get the color of a given pixel
    fn get_pixel(&self, col: usize, row: usize) -> Color {
        self.pixels[row][col].clone()
    }

    /// Set the color of a given pixel
    pub fn set_pixel(&mut self, col: usize, row: usize, color: &Color) {
        self.pixels[row][col] = color.clone();
    }

    pub fn draw<D: Drawable>(&mut self, drawable: D, color: &Color) {
        for (col, row) in drawable.coordinates() {
            self.set_pixel(col, row, &color);
        }
    }

    /// Get the height of the canvas
    pub fn height(&self) -> usize {
        self.height
    }

    /// Get the width of the canvas
    pub fn width(&self) -> usize {
        self.width
    }
}



pub struct Inky {
    display: Box<dyn InkyDisplay>,
    canvas: Canvas,
}

impl Inky {
    pub fn canvas(&self) -> &Canvas {
        &self.canvas
    }

    pub fn canvas_mut(&mut self) -> &mut Canvas {
        &mut self.canvas
    }

    pub fn update(&mut self) -> Result<()> {
        let buf = self.display.convert(&self.canvas.pixels)?;
        self.display.update(buf)
    }

    /// Refresh a rectangular region of the canvas when the display supports it.
    ///
    /// Coordinates are measured from the canvas's top-left corner. Partial updates are
    /// currently supported only by the Inky E673.
    pub fn update_region(
        &mut self,
        x: usize,
        y: usize,
        width: usize,
        height: usize,
    ) -> Result<()> {
        ensure!(width > 0 && height > 0, "Update region must not be empty");
        let x_end = x
            .checked_add(width)
            .ok_or_else(|| anyhow::anyhow!("Update region x-coordinate overflow"))?;
        let y_end = y
            .checked_add(height)
            .ok_or_else(|| anyhow::anyhow!("Update region y-coordinate overflow"))?;
        ensure!(
            x_end <= self.canvas.width && y_end <= self.canvas.height,
            "Update region is outside the canvas bounds"
        );

        let pixels = self.canvas.pixels[y..y_end]
            .iter()
            .map(|row| row[x..x_end].to_vec())
            .collect();
        self.display.update_region(x, y, pixels)
    }
    
}

impl TryFrom<EEPROM> for Inky {
    type Error = Error;

    fn try_from(value: EEPROM) -> Result<Self> {
        print!("Creating Inky display of type {:?}\n", value.display_variant());
        print!("Display dimensions: {}x{}\n", value.width(), value.height());
        let canvas = Canvas::new(value.width() as usize, value.height() as usize);
        match value.display_variant() {
            DisplayVariant::E673 => Ok(Self {
                display: Box::new(InkyE673::new_with_speed(value, RefreshSpeed::Fast)?),
                canvas,
            }),
            DisplayVariant::What => Ok(Self {
                display: Box::new(InkyWhat::new(value)?),
                canvas,
            }),
            DisplayVariant::PhatSsd1608 => Ok(Self {
                display: Box::new(InkyPhatSsd1608::new(value)?),
                canvas,
            }),
            _ => bail!("Unsupported display variant"),
        }   
    } 
}


#[cfg(test)]
mod tests {

    use super::{Inky, Rectangle};
    use crate::eeprom::EEPROM;
    use crate::core::colors::Color;
    use anyhow::Result;

    #[test]
    fn test_blank() -> Result<()> {
        let eeprom = EEPROM::try_new().expect("Failed to initialize eeprom");
        let mut inky = Inky::try_from(eeprom)?;
        inky.update()?;
        Ok(())
    }

    #[test]
    fn test_draw_box() -> Result<()> {
        let eeprom = EEPROM::try_new().expect("Failed to initialize eeprom");
        let mut inky = Inky::try_from(eeprom)?;

        inky.canvas_mut().draw(Rectangle::new((20, 20), (780, 460)), &Color::Black);

        inky.update()?;
        Ok(())
    }
}
