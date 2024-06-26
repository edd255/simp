/// This crate contains the data structure that represents images as pixel matrices and
/// functionalities as cropping, rotating, inverting and seam carving.

pub mod image {
    use crate::energy_utils::energy;
    use crate::pixel_utils::pixel::Pixel;
    use nalgebra::DMatrix;
    use std::borrow::Cow;
    use std::fmt::Write as OtherWrite;
    use std::fs;
    use std::io::Write;

    /// Images in the PPM format have a `magic_number`, e.g. P3 for Portable Pixmaps (ASCII), and a
    /// `scale` is the maximum value for each color. Images are represented as pixel matrices, here
    /// in `pixels`.
    pub struct Image {
        pub magic_number: String,
        pub scale: u8,
        pub pixels: DMatrix<Pixel>,
    }

    impl Image {
        //=== READING & WRITING ===================================================================

        /// Returns an image struct, parsed from a file
        ///
        /// # Parameters:
        ///  `file` - The location of the file, as a String
        ///
        /// # Returns:
        ///  `Image` - Representation of the image file with the struct Image
        pub fn read(file: &String) -> Image {
            let contents = match fs::read_to_string(file) {
                Ok(str) => str,
                Err(err) => panic!("{err:?}"),
            };
            let mut lines = contents.lines();
            let header: Vec<&str> = lines.by_ref().take(3).collect();
            let body_str: Vec<String> = lines
                .map(|line| Cow::<str>::Owned(line.replace('\n', " ")).into_owned())
                .collect();
            let body: Vec<&str> = body_str.iter().map(std::string::String::as_str).collect();
            let Some((magic_number, width, height, scale)) = Self::parse_header(&header) else {
                panic!("Error in parsing the header")
            };
            let pixels: DMatrix<Pixel> = match Self::parse_pixels(&body, width, height) {
                Ok(pixels) => pixels,
                Err(e) => panic!("{e:?}"),
            };
            Image {
                magic_number,
                scale,
                pixels,
            }
        }

        /// Parse the header of a PPM image file.
        ///
        /// # Parameters:
        ///  `lines` - The lines to parse
        ///
        /// # Returns:
        ///  `Option<(String, usize, usize, u8)>` - Parse the magic number and the dimensions of the file.
        fn parse_header(lines: &[&str]) -> Option<(String, usize, usize, u8)> {
            let magic_number = lines.first().unwrap();
            let dimensions: Vec<&str> = match lines.get(1) {
                Some(dimensions) => dimensions.split(' ').collect(),
                None => return None,
            };
            let width = dimensions[0].parse::<usize>().unwrap();
            let height = dimensions[1].parse::<usize>().unwrap();
            let scale: u8 = match lines.get(2) {
                Some(size) => size.parse::<u8>().unwrap(),
                None => return None,
            };
            Some(((*magic_number).to_string(), width, height, scale))
        }

        /// Parse the pixels of the PPM image file.
        ///
        /// # Parameters:
        ///  lines - The lines to parse
        ///
        /// # Returns:
        ///  `Option<Vec<Pixel>>`-  Returns an Optional of a pixel matrix, saved as vector
        fn parse_pixels(
            lines: &[&str],
            width: usize,
            height: usize,
        ) -> Result<DMatrix<Pixel>, &'static str> {
            let data: String = lines
                .iter()
                .fold(String::new(), |mut acc, line| {
                    acc.push_str(&format!("{line} "));
                    acc
                })
                .chars()
                .collect();
            let values: Vec<&str> = data.split_whitespace().collect();
            if values.len() < width * height * 3 {
                println!("Insufficient data for the specified dimensions");
            }
            let mut pixels = Vec::new();
            for chunk in values.chunks(3) {
                if let [r, g, b] = chunk {
                    let red: u8 = r.parse().map_err(|_| "Failed to parse red component")?;
                    let green: u8 = g.parse().map_err(|_| "Failed to parse green component")?;
                    let blue: u8 = b.parse().map_err(|_| "Failed to parse blue component")?;
                    pixels.push(Pixel { red, green, blue });
                } else {
                    return Err("Invalid pixel data");
                }
            }
            let mut matrix = DMatrix::zeros(height, width);
            for (idx, pixel) in pixels.into_iter().enumerate() {
                let row = idx / width;
                let col = idx % width;
                matrix[(row, col)] = pixel;
            }
            Ok(matrix)
        }

        /// Write an image to a file.
        ///
        /// # Parameters:
        ///  `filename` - path to the file
        pub fn write(&self, filename: &String) {
            let mut file = fs::File::create(filename).expect("Could not write to file");
            writeln!(file, "{}", self.magic_number).expect("Could not write magic number.");
            writeln!(file, "{} {}", self.pixels.ncols(), self.pixels.nrows())
                .expect("Could not write height and width.");
            writeln!(file, "{}", self.scale).expect("Could not write scale");
            let mut buffer = String::new();
            for y in 0..self.pixels.nrows() {
                for x in 0..self.pixels.ncols() {
                    let pixel = &self.pixels[(y, x)];
                    let red = pixel.red;
                    let green = pixel.green;
                    let blue = pixel.blue;
                    write!(buffer, "{red:3} {green:3} {blue:3} ").expect("Could not write pixel");
                }
                writeln!(buffer).expect("Could not write newline");
            }
            file.write_all(buffer.as_bytes())
                .expect("Could not write buffer to file");
            buffer.clear();
        }

        //=== IMAGE STATISTICS ====================================================================

        /// Returns the brightness of the pixels, defined as the sum of the color channels, divided
        /// by three.
        ///
        /// # Returns:
        ///  `u32`-  Brightness of the image
        fn brightness(&self) -> u32 {
            let size: u32 = (self.pixels.nrows() * self.pixels.ncols())
                .try_into()
                .unwrap();
            let mut sum: u32 = 0;
            for pixel in &self.pixels {
                sum += (u32::from(pixel.red) + u32::from(pixel.green) + u32::from(pixel.blue)) / 3;
            }
            sum / size
        }

        /// Print statistics from the image.
        pub fn statistics(&self) {
            println!("Type:       {}", self.magic_number);
            println!("Height:     {}", self.pixels.nrows());
            println!("Width:      {}", self.pixels.ncols());
            println!("Brightness: {}", self.brightness());
        }

        //=== SEAM CARVING ========================================================================

        /// Seam carves an image using the following procedure:
        ///     * Calculate the appropriate energy matrix.
        ///     * Find the pixel with the minimal energy at the width/height up to which the energy
        ///     is calculated to.
        ///     * Calculate the seam.
        ///     * Carve the seam.
        ///
        /// # Parameters
        ///  `iterations` - how many seams should be removed
        ///  `output` - where the output image should be stored
        ///  `vertical` - whether vertical or horizontal seams should be removed
        pub fn seam_carve(&mut self, iterations: usize, output: &String, vertical: bool) {
            if vertical {
                let width = self.pixels.ncols();
                let mut border = self.pixels.ncols();
                let mut energy_matrix: DMatrix<u32> =
                    DMatrix::from_element(self.pixels.nrows(), self.pixels.ncols(), 0);
                for _ in 0..iterations {
                    energy::calculate_vertical_energy_matrix(self, &mut energy_matrix, width);
                    let x = energy::calculate_min_energy_column(&energy_matrix, border);
                    let seam = energy::calculate_optimal_vertical_path(&energy_matrix, border, x);
                    self.carve_vertical_path(border, &seam);
                    border -= 1;
                }
                self.crop(output, 0, width - iterations, 0, self.pixels.nrows());
            } else {
                let height = self.pixels.nrows();
                let mut border = self.pixels.nrows();
                let mut energy_matrix: DMatrix<u32> =
                    DMatrix::from_element(self.pixels.nrows(), self.pixels.ncols(), 0);
                for _ in 0..iterations {
                    energy::calculate_horizontal_energy_matrix(self, &mut energy_matrix, height);
                    let x = energy::calculate_min_energy_row(&energy_matrix, border);
                    let seam = energy::calculate_optimal_horizontal_path(&energy_matrix, border, x);
                    self.carve_horizontal_path(border, &seam);
                    border -= 1;
                }
                self.crop(output, 0, self.pixels.ncols(), 0, height - iterations);
            }
        }

        /// Carves a vertical path.
        ///
        /// # Parameters
        ///  `border` - the width up to which the energy matrix is calculated to
        ///  `seam` - the seam to carve
        fn carve_vertical_path(&mut self, border: usize, seam: &[usize]) {
            for j in 0..self.pixels.nrows() {
                let col = *seam.get(j).unwrap();
                for i in col..border - 1 {
                    self.pixels[(j, i)].red = self.pixels[(j, i + 1)].red;
                    self.pixels[(j, i)].green = self.pixels[(j, i + 1)].green;
                    self.pixels[(j, i)].blue = self.pixels[(j, i + 1)].blue;
                }
            }
        }

        /// Carves a horizontal path.
        ///
        /// # Parameters
        ///  `border` - the height up to which the energy matrix is calculated to
        ///  `seam` - the seam to carve
        fn carve_horizontal_path(&mut self, border: usize, seam: &[usize]) {
            for j in 0..self.pixels.ncols() {
                let row = *seam.get(j).unwrap();
                for i in row..border - 1 {
                    self.pixels[(i, j)].red = self.pixels[(i + 1, j)].red;
                    self.pixels[(i, j)].green = self.pixels[(i + 1, j)].green;
                    self.pixels[(i, j)].blue = self.pixels[(i + 1, j)].blue;
                }
            }
        }

        //=== IMAGE MANIPULATION ==================================================================

        /// Crop an image
        ///
        /// # Parameters:
        ///  `filename` - path to the file (as String)
        ///  `x1` - lower vertical border
        ///  `x2` - upper vertical border
        ///  `y1` - left horizontal border
        ///  `y2` - right horizontal border
        pub fn crop(&self, filename: &String, x1: usize, x2: usize, y1: usize, y2: usize) {
            assert!(x1 <= self.pixels.ncols());
            assert!(x2 <= self.pixels.ncols());
            assert!(y1 <= self.pixels.nrows());
            assert!(y2 <= self.pixels.nrows());
            let mut file = fs::File::create(filename).expect("Could not write to file");
            writeln!(file, "{}", self.magic_number).expect("Could not write magic number.");
            writeln!(file, "{} {}", x2 - x1, y2 - y1).expect("Could not write height and width.");
            writeln!(file, "{}", self.scale).expect("Could not write scale");
            let mut buffer = String::new();
            for y in y1..y2 {
                for x in x1..x2 {
                    let pixel = &self.pixels[(y, x)];
                    let red = pixel.red;
                    let green = pixel.green;
                    let blue = pixel.blue;
                    write!(buffer, "{red:3} {green:3} {blue:3} ").expect("Could not write pixel");
                }
                writeln!(buffer).expect("Could not write newline");
            }
            file.write_all(buffer.as_bytes())
                .expect("Could not write buffer to file");
            buffer.clear();
        }

        /// Transposes an image.
        ///
        /// Parameters:
        ///  `filename` - Path to the file
        pub fn transpose(&self, filename: &String) {
            let mut file = fs::File::create(filename).expect("Could not write to file");
            writeln!(file, "{}", self.magic_number).expect("Could not write magic number.");
            writeln!(file, "{} {}", self.pixels.nrows(), self.pixels.ncols())
                .expect("Could not write height and width.");
            writeln!(file, "{}", self.scale).expect("Could not write scale");
            let mut buffer = String::new();
            for x in 0..self.pixels.ncols() {
                for y in 0..self.pixels.nrows() {
                    let pixel = &self.pixels[(y, x)];
                    let red = pixel.red;
                    let green = pixel.green;
                    let blue = pixel.blue;
                    write!(buffer, "{red:3} {green:3} {blue:3} ").expect("Could not write pixel");
                }
                writeln!(buffer).expect("Could not write newline");
            }
            file.write_all(buffer.as_bytes())
                .expect("Could not write buffer to file");
            buffer.clear();
        }

        /// Rotates an image.
        ///
        /// Parameters:
        ///  `filename` - Path to the file
        pub fn rotate(&self, filename: &String) {
            let mut file = fs::File::create(filename).expect("Could not write to file");
            writeln!(file, "{}", self.magic_number).expect("Could not write magic number.");
            writeln!(file, "{} {}", self.pixels.nrows(), self.pixels.ncols())
                .expect("Could not write height and width.");
            writeln!(file, "{}", self.scale).expect("Could not write scale");
            let mut buffer = String::new();
            for x in 0..self.pixels.ncols() {
                for y in 0..self.pixels.nrows() {
                    let pixel = &self.pixels[(self.pixels.nrows() - 1 - y, x)];
                    let red = pixel.red;
                    let green = pixel.green;
                    let blue = pixel.blue;
                    write!(buffer, "{red:3} {green:3} {blue:3} ").expect("Could not write pixel");
                }
                writeln!(buffer).expect("Could not write newline");
            }
            file.write_all(buffer.as_bytes())
                .expect("Could not write buffer to file");
            buffer.clear();
        }

        /// Rotate an image.
        ///
        /// # Parameters:
        ///  `filename` - Path to the output file
        pub fn invert(&mut self, filename: &String) {
            let mut file = fs::File::create(filename).expect("Could not write to file");
            writeln!(file, "{}", self.magic_number).expect("Could not write magic number");
            writeln!(file, "{} {}", self.pixels.ncols(), self.pixels.nrows())
                .expect("Could not write height and width.");
            writeln!(file, "{}", self.scale).expect("Could not write scale");
            let mut buffer = String::new();
            for y in 0..self.pixels.nrows() {
                for x in 0..self.pixels.ncols() {
                    let pixel = &mut self.pixels[(y, x)];
                    pixel.invert();
                    let red = pixel.red;
                    let green = pixel.green;
                    let blue = pixel.blue;
                    write!(buffer, "{red:3} {green:3} {blue:3} ").expect("Could not write pixel");
                }
                writeln!(buffer).expect("Could not write newline");
            }
            file.write_all(buffer.as_bytes())
                .expect("Could not write buffer to file");
            buffer.clear();
        }

        /// Mirror an image
        ///
        /// # Parameters:
        ///  `filename` - path to the file (as String)
        pub fn mirror(&self, filename: &String) {
            let mut file = fs::File::create(filename).expect("Could not write to file");
            writeln!(file, "{}", self.magic_number).expect("Could not write magic number.");
            writeln!(file, "{} {}", self.pixels.ncols(), self.pixels.nrows())
                .expect("Could not write height and width.");
            writeln!(file, "{}", self.scale).expect("Could not write scale");
            let mut buffer = String::new();
            for y in 0..self.pixels.nrows() {
                for x in 0..self.pixels.ncols() {
                    let pixel = &self.pixels[(y, self.pixels.ncols() - 1 - x)];
                    let red = pixel.red;
                    let green = pixel.green;
                    let blue = pixel.blue;
                    write!(buffer, "{red:3} {green:3} {blue:3} ").expect("Could not write pixel");
                }
                writeln!(buffer).expect("Could not write newline");
            }
            file.write_all(buffer.as_bytes())
                .expect("Could not write buffer to file");
            buffer.clear();
        }

        /// Landfill using a color and a point
        ///
        /// # Parameters:
        ///  `filename` - path to the file (as String)
        ///  `coords` - x and y coordinaates
        ///  `rgb` - red, green and blue pixel values
        pub fn landfill(&mut self, filename: &String, coords: (usize, usize), rgb: (u8, u8, u8)) {
            env_logger::init();
            let (y, x) = coords;
            let (red, green, blue) = rgb;
            if x >= self.pixels.ncols() && y >= self.pixels.nrows() {
                return;
            }
            let original_point = (
                self.pixels[(y, x)].red,
                self.pixels[(y, x)].green,
                self.pixels[(y, x)].blue,
            );
            let mut stack: Vec<(usize, usize)> = vec![];
            stack.push((y, x));
            while !stack.is_empty() {
                let Some((y1, x1)) = stack.pop() else {
                    break;
                };
                let mut px = self.pixels[(y1, x1)];
                if Self::inside(original_point, px) {
                    self.pixels[(y1, x1)].red = red;
                    self.pixels[(y1, x1)].green = green;
                    self.pixels[(y1, x1)].blue = blue;
                }
                if x1 + 1 < self.pixels.ncols() && y1 < self.pixels.nrows() {
                    px = self.pixels[(y1, x1 + 1)];
                    if Self::inside(original_point, px) {
                        stack.push((y1, x1 + 1));
                    }
                }
                if x1 - 1 < self.pixels.ncols() && y1 < self.pixels.nrows() {
                    px = self.pixels[(y1, x1 - 1)];
                    if Self::inside(original_point, px) {
                        stack.push((y1, x1 - 1));
                    }
                }
                if x1 < self.pixels.ncols() && y1 + 1 < self.pixels.nrows() {
                    px = self.pixels[(y1 + 1, x1)];
                    if Self::inside(original_point, px) {
                        stack.push((y1 + 1, x1));
                    }
                }
                if x1 < self.pixels.ncols() && y1 - 1 < self.pixels.nrows() {
                    px = self.pixels[(y1 - 1, x1)];
                    if Self::inside(original_point, px) {
                        stack.push((y1 - 1, x1));
                    }
                }
            }
            self.write(filename);
        }

        /// Checks whether the pixel has the required colors.
        ///
        /// # Parameters
        ///  `rgb` - red, green and blue pixel values
        ///  `pixel` - the pixel to inspect
        ///
        /// # Returns
        ///  true if the pixel has the required colors
        fn inside(rgb: (u8, u8, u8), pixel: Pixel) -> bool {
            rgb.0 == pixel.red && rgb.1 == pixel.green && rgb.2 == pixel.blue
        }
    }
}
