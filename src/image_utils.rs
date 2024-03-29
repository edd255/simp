/// This crate contains the data structure that represents images as pixel matrices and
/// functionalities as cropping, rotating, inverting and seam carving.

pub mod image {
    use crate::energy_utils::energy;
    use crate::pixel_utils::pixel::Pixel;
    use nalgebra::DMatrix;
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
        /// # Source:
        ///   * <https://github.com/chris-paterson/PPM>
        ///
        /// # Parameters:
        ///   `file` - The location of the file, as a String
        ///
        /// # Returns:
        ///   `Image` - Representation of the image file with the struct Image
        pub fn read(file: &String) -> Image {
            let contents = match fs::read_to_string(file) {
                Ok(str) => str,
                Err(err) => panic!("{err:?}"),
            };
            let lines: Vec<&str> = contents.split('\n').collect();
            let header: Vec<&str> = lines[0..3].to_vec();
            let body: Vec<&str> = lines[3..].to_vec();
            let (magic_number, width, height, scale) = match Self::parse_header(&header) {
                Some((m, w, h, s)) => (m, w, h, s),
                None => panic!("Error in parsing the header"),
            };
            let pixels: DMatrix<Pixel> = match Self::parse_pixels(&body, width, height) {
                Some(pixels) => pixels,
                None => panic!("Error in parsing the pixels."),
            };
            Image {
                magic_number,
                scale,
                pixels,
            }
        }

        /// Parse the header of a PPM image file.
        ///
        /// # Source:
        ///   * <https://github.com/chris-paterson/PPM>
        ///
        /// # Parameters:
        ///   `lines` - The lines to parse
        ///
        /// # Returns:
        ///   `Option<(String, usize, usize, u8)>` - Parse the magic number and the dimensions of the file.
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
        /// # Source:
        ///   * <https://github.com/chris-paterson/PPM>
        ///
        /// # Parameters:
        ///   lines - The lines to parse
        ///
        /// # Returns:
        ///   `Option<Vec<Pixel>>`-  Returns an Optional of a pixel matrix, saved as vector
        fn parse_pixels(lines: &[&str], width: usize, height: usize) -> Option<DMatrix<Pixel>> {
            if width == 0 || height == 0 {
                return None;
            }
            let mut pixels = Vec::new();
            for line in lines.iter().take(height) {
                let mut parts = line.split_whitespace();
                for _ in 0..width {
                    let red: u8 = parts.next()?.parse().ok()?;
                    let green: u8 = parts.next()?.parse().ok()?;
                    let blue: u8 = parts.next()?.parse().ok()?;
                    pixels.push(Pixel { red, green, blue });
                }
            }
            if pixels.len() != width * height {
                return None;
            }
            let mut matrix = DMatrix::zeros(height, width);
            for (idx, pixel) in pixels.into_iter().enumerate() {
                let row = idx / width;
                let col = idx % width;
                matrix[(row, col)] = pixel;
            }
            Some(matrix)
        }

        /// Write an image to a file.
        ///
        /// # Source:
        ///   * <https://github.com/chris-paterson/PPM>
        ///
        /// # Parameters:
        ///   `filename` - path to the file
        pub fn write(&self, filename: &String) {
            let mut file = fs::File::create(filename).expect("Could not write to file");
            writeln!(file, "{}", self.magic_number).expect("Could not write magic number.");
            writeln!(file, "{} {}", self.pixels.ncols(), self.pixels.nrows())
                .expect("Could not write height and width.");
            writeln!(file, "{}", self.scale).expect("Could not write scale");
            for y in 0..self.pixels.nrows() {
                for x in 0..self.pixels.ncols() {
                    let pixel = &self.pixels[(x, y)];
                    let red = pixel.red;
                    let green = pixel.green;
                    let blue = pixel.blue;
                    write!(file, "{red} {green} {blue}").expect("Could not write pixel");
                }
                writeln!(file).expect("Could not write newline");
            }
        }

        //=== IMAGE STATISTICS ====================================================================

        /// Returns the brightness of the pixels, defined as the sum of the color channels, divided
        /// by three.
        ///
        /// # Returns:
        ///   `u32`-  Brightness of the image
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

        pub fn seam_carve(&mut self, iterations: usize, output: &String) {
            let mut border = self.pixels.ncols();
            for _ in 0..iterations {
                let energy_matrix = energy::calculate_energy(self, border);
                let x = energy::calculate_min_energy_column(&energy_matrix, border);
                let seam = energy::calculate_optimal_path(&energy_matrix, border, x);
                self.carve_path(&border, &seam);
                border -= 1;
            }
            self.crop(output, border);
        }

        fn carve_path(&mut self, border: &usize, seam: &[usize]) {
            for j in 0..self.pixels.nrows() {
                let col = *seam.get(j).unwrap();
                for i in col..border - 1 {
                    self.pixels[(j, i)].red = self.pixels[(j, i + 1)].red;
                    self.pixels[(j, i)].green = self.pixels[(j, i + 1)].green;
                    self.pixels[(j, i)].blue = self.pixels[(j, i + 1)].blue;
                }
            }
        }

        //=== IMAGE MANIPULATION ==================================================================

        /// Crop an image
        ///
        /// # Parameters:
        ///   `filename` - path to the file (as String)
        pub fn crop(&self, filename: &String, border: usize) {
            assert!(border <= self.pixels.nrows());
            let mut file = fs::File::create(filename).expect("Could not write to file");
            writeln!(file, "{}", self.magic_number).expect("Could not write magic number.");
            writeln!(file, "{} {}", self.pixels.ncols(), self.pixels.nrows())
                .expect("Could not write height and width.");
            writeln!(file, "{}", self.scale).expect("Could not write scale");
            for y in 0..border {
                for x in 0..self.pixels.ncols() {
                    let pixel = &self.pixels[(x, y)];
                    let red = pixel.red;
                    let green = pixel.green;
                    let blue = pixel.blue;
                    write!(file, "{red} {green} {blue}").expect("Could not write pixel");
                }
                writeln!(file).expect("Could not write newline");
            }
        }

        /// Rotate an image.
        ///
        /// Parameters:
        ///   `filename` - Path to the file
        pub fn rotate(&self, filename: &String) {
            let mut file = fs::File::create(filename).expect("Could not write to file");
            writeln!(file, "{}", self.magic_number).expect("Could not write magic number.");
            writeln!(file, "{} {}", self.pixels.ncols(), self.pixels.nrows())
                .expect("Could not write height and width.");
            writeln!(file, "{}", self.scale).expect("Could not write scale");
            for y in 0..self.pixels.nrows() {
                for x in 0..self.pixels.ncols() {
                    let pixel = &self.pixels[(x, y)];
                    let red = pixel.red;
                    let green = pixel.green;
                    let blue = pixel.blue;
                    write!(file, "{red} {green} {blue}").expect("Could not write pixel");
                }
                writeln!(file).expect("Could not write newline");
            }
        }

        /// Rotate an image.
        ///
        /// # Parameters:
        ///   `filename` - Path to the output file
        pub fn invert(&mut self, filename: &String) {
            let mut file = fs::File::create(filename).expect("Could not write to file");
            writeln!(file, "{}", self.magic_number).expect("Could not write magic number");
            writeln!(file, "{} {}", self.pixels.ncols(), self.pixels.nrows())
                .expect("Could not write height and width.");
            writeln!(file, "{}", self.scale).expect("Could not write scale");
            for y in 0..self.pixels.nrows() {
                for x in 0..self.pixels.ncols() {
                    let pixel = &mut self.pixels[(x, y)];
                    pixel.invert();
                    let red = pixel.red;
                    let green = pixel.green;
                    let blue = pixel.blue;
                    write!(file, "{red}, {green} {blue}").expect("Could not write pixel");
                }
                writeln!(file).expect("Could not write newline");
            }
        }
    }
}
