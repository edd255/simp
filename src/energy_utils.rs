/// Seam Carving uses color differences of neighboring pixels as dispensability score. This
/// difference is called energy. This crate contains methods to calculate the energy of an image
/// and to find the optimal path according to this dispensability score.

pub mod energy {
    use crate::image_utils::image::Image;
    use crate::pixel_utils::pixel::Pixel;
    use nalgebra::DMatrix;
    use std::cmp::min;

    pub fn calculate_energy_matrix(image: &Image, energy: &mut DMatrix<u32>, border: usize) {
        // Calculation of local energy
        // Edge Case: First Element
        energy[(0, 0)] = 0;
        // Edge Case: First Row
        for j in 1..border {
            let current = (0, j);
            let left = (0, j - 1);
            energy[current] = Pixel::color_diff(image.pixels[current], image.pixels[left]);
        }
        // Edge Case: Left Border
        for i in 1..image.pixels.nrows() {
            let current = (i, 0);
            let above = (i - 1, 0);
            energy[current] = Pixel::color_diff(image.pixels[current], image.pixels[above]);
        }
        // No Edge Cases
        for i in 1..image.pixels.nrows() {
            for j in 1..border {
                let current = (i, j);
                let left = (i, j - 1);
                let above = (i - 1, j);
                energy[current] = Pixel::color_diff(image.pixels[current], image.pixels[left])
                    + Pixel::color_diff(image.pixels[current], image.pixels[above]);
            }
        }
        // Calculation of total energy
        for i in 1..image.pixels.nrows() {
            for j in 0..border {
                let current = (i, j);
                let left = (i - 1, j - 1);
                let above = (i - 1, j);
                let right = (i - 1, j + 1);
                if j == 0 {
                    // Edge Case: Left Border
                    energy[current] += min(energy[above], energy[right]);
                } else if j == border - 1 {
                    // Edge Case: Right Border
                    energy[current] += min(energy[above], energy[left]);
                } else {
                    // No Edge Cases
                    energy[current] += min(min(energy[above], energy[left]), energy[right]);
                }
            }
        }
    }

    pub fn calculate_min_energy_column(energy: &DMatrix<u32>, border: usize) -> usize {
        let mut column: usize = 0;
        for i in 1..border {
            if energy[(energy.nrows() - 1, column)] > energy[(energy.nrows() - 1, i)] {
                column = i;
            }
        }
        column
    }

    pub fn calculate_min_energy_row(energy: &DMatrix<u32>, border: usize) -> usize {
        let mut row: usize = 0;
        for i in 1..border {
            if energy[(row, energy.ncols() - 1)] > energy[(i, energy.ncols() - 1)] {
                row = i;
            }
        }
        row
    }

    pub fn calculate_optimal_vertical_path(
        energy: &DMatrix<u32>,
        border: usize,
        start: usize,
    ) -> Vec<usize> {
        let mut seam = vec![0; energy.nrows()];
        seam[energy.nrows() - 1] = start;
        for j in (1..energy.nrows()).rev() {
            let left = (j - 1, seam[j] - 1);
            let above = (j - 1, seam[j]);
            let right = (j - 1, seam[j] + 1);
            if seam[j] == 0 {
                // Case: Left border
                if energy[above] <= energy[right] {
                    seam[j - 1] = seam[j];
                } else {
                    seam[j - 1] = seam[j] + 1;
                }
            } else if seam[j] == border - 1 {
                // Case: Right Border
                if energy[above] <= energy[left] {
                    seam[j - 1] = seam[j];
                } else {
                    seam[j - 1] = seam[j] - 1;
                }
            } else if energy[above] == energy[left] {
                // Precedence for multiple optimal pixels
                if energy[above] <= energy[right] {
                    seam[j - 1] = seam[j];
                } else {
                    seam[j - 1] = seam[j] + 1;
                }
            } else if energy[above] <= energy[right] {
                if energy[above] <= energy[left] {
                    seam[j - 1] = seam[j];
                } else {
                    seam[j - 1] = seam[j] - 1;
                }
            } else {
                // Remainder
                if energy[left] < energy[above] && energy[left] <= energy[right] {
                    seam[j - 1] = seam[j] - 1;
                } else if energy[above] < energy[left] && energy[above] <= energy[right] {
                    seam[j - 1] = seam[j];
                } else {
                    seam[j - 1] = seam[j] + 1;
                }
            }
        }
        seam
    }

    pub fn calculate_optimal_horizontal_path(
        energy: &DMatrix<u32>,
        border: usize,
        start: usize,
    ) -> Vec<usize> {
        let mut seam = vec![0; energy.ncols()];
        seam[energy.ncols() - 1] = start;
        for j in (1..energy.ncols()).rev() {
            let left = (seam[j] - 1, j - 1);
            let above = (seam[j], j - 1);
            let right = (seam[j] + 1, j - 1);
            if seam[j] == 0 {
                // Case: Left border
                if energy[above] <= energy[right] {
                    seam[j - 1] = seam[j];
                } else {
                    seam[j - 1] = seam[j] + 1;
                }
            } else if seam[j] == border - 1 {
                // Case: Right Border
                if energy[above] <= energy[left] {
                    seam[j - 1] = seam[j];
                } else {
                    seam[j - 1] = seam[j] - 1;
                }
            } else if energy[above] == energy[left] {
                // Precedence for multiple optimal pixels
                if energy[above] <= energy[right] {
                    seam[j - 1] = seam[j];
                } else {
                    seam[j - 1] = seam[j] + 1;
                }
            } else if energy[above] <= energy[right] {
                if energy[above] <= energy[left] {
                    seam[j - 1] = seam[j];
                } else {
                    seam[j - 1] = seam[j] - 1;
                }
            } else {
                // Remainder
                if energy[left] < energy[above] && energy[left] <= energy[right] {
                    seam[j - 1] = seam[j] - 1;
                } else if energy[above] < energy[left] && energy[above] <= energy[right] {
                    seam[j - 1] = seam[j];
                } else {
                    seam[j - 1] = seam[j] + 1;
                }
            }
        }
        seam
    }
}
