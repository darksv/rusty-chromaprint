use crate::filter::Image;

pub struct RollingIntegralImage {
    max_rows: usize,
    columns: usize,
    rows: usize,
    data: Vec<f64>,
}

impl RollingIntegralImage {
    pub fn new(max_rows: usize) -> Self {
        Self {
            max_rows: max_rows + 1,
            columns: 0,
            rows: 0,
            data: Vec::new(),
        }
    }

    #[cfg(test)]
    pub fn from_data<D>(columns: usize, data: &[D]) -> Self
    where
        D: Copy + Into<f64>,
    {
        let mut image = Self {
            max_rows: data.len() / columns,
            columns: 0,
            rows: 0,
            data: Vec::with_capacity(data.len()),
        };

        for row in data.chunks_exact(columns) {
            image.add_row(row);
        }

        image
    }

    pub(crate) fn add_row<T>(&mut self, row: &[T])
    where
        T: Copy + Into<f64>,
    {
        if self.columns == 0 {
            self.columns = row.len();
            self.data.resize(self.max_rows * self.columns, 0.0);
        }

        assert_eq!(self.columns, row.len());

        let mut sum = 0.0;
        for (i, &cell) in row.iter().enumerate().take(self.columns) {
            sum += cell.into();
            self.row_mut(self.rows)[i] = sum;
        }

        if self.rows > 0 {
            for i in 0..self.columns {
                self.row_mut(self.rows)[i] += self.row(self.rows - 1)[i];
            }
        }

        self.rows += 1;
    }

    #[cfg(test)]
    fn columns(&self) -> usize {
        self.columns
    }

    pub(crate) fn rows(&self) -> usize {
        self.rows
    }

    fn row(&self, mut i: usize) -> &[f64] {
        i %= self.max_rows;
        &self.data[i * self.columns..][..self.columns]
    }

    fn row_mut(&mut self, mut i: usize) -> &mut [f64] {
        i %= self.max_rows;
        &mut self.data[i * self.columns..][..self.columns]
    }

    pub(crate) fn reset(&mut self) {
        self.data.clear();
        self.rows = 0;
        self.columns = 0;
    }
}

impl Image for RollingIntegralImage {
    fn area(&self, r1: usize, c1: usize, r2: usize, c2: usize) -> f64 {
        assert!(r1 <= self.rows);
        assert!(r2 <= self.rows);

        if self.rows > self.max_rows {
            assert!(r1 > self.rows - self.max_rows);
            assert!(r2 > self.rows - self.max_rows);
        }

        assert!(c1 <= self.columns);
        assert!(c2 <= self.columns);

        if r1 == r2 || c1 == c2 {
            return 0.0;
        }

        assert!(r2 > r1);
        assert!(c2 > c1);

        if r1 == 0 {
            let row = self.row(r2 - 1);
            if c1 == 0 {
                row[c2 - 1]
            } else {
                row[c2 - 1] - row[c1 - 1]
            }
        } else {
            let row1 = self.row(r1 - 1);
            let row2 = self.row(r2 - 1);
            if c1 == 0 {
                row2[c2 - 1] - row1[c2 - 1]
            } else {
                row2[c2 - 1] - row1[c2 - 1] - row2[c1 - 1] + row1[c1 - 1]
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::assert_eq_float;
    use crate::filter::Image;
    use crate::rolling_image::RollingIntegralImage;

    #[test]
    fn simple() {
        let mut image = RollingIntegralImage::new(4);
        image.add_row(&[1, 2, 3]);

        assert_eq!(3, image.columns());
        assert_eq!(1, image.rows());

        assert_eq_float!(1.0, image.area(0, 0, 1, 1));
        assert_eq_float!(2.0, image.area(0, 1, 1, 2));
        assert_eq_float!(3.0, image.area(0, 2, 1, 3));
        assert_eq_float!(1.0 + 2.0 + 3.0, image.area(0, 0, 1, 3));

        image.add_row(&[4, 5, 6]);

        assert_eq!(3, image.columns());
        assert_eq!(2, image.rows());

        assert_eq_float!(4.0, image.area(1, 0, 2, 1));
        assert_eq_float!(5.0, image.area(1, 1, 2, 2));
        assert_eq_float!(6.0, image.area(1, 2, 2, 3));
        assert_eq_float!(1.0 + 2.0 + 3.0 + 4.0 + 5.0 + 6.0, image.area(0, 0, 2, 3));

        image.add_row(&[7, 8, 9]);

        assert_eq!(3, image.columns());
        assert_eq!(3, image.rows());

        image.add_row(&[10, 11, 12]);

        assert_eq!(3, image.columns());
        assert_eq!(4, image.rows());

        assert_eq_float!(
            (1.0 + 2.0 + 3.0) + (4.0 + 5.0 + 6.0) + (7.0 + 8.0 + 9.0) + (10.0 + 11.0 + 12.0),
            image.area(0, 0, 4, 3)
        );

        image.add_row(&[13, 14, 15]);

        assert_eq!(3, image.columns());
        assert_eq!(5, image.rows());

        assert_eq_float!(4.0, image.area(1, 0, 2, 1));
        assert_eq_float!(5.0, image.area(1, 1, 2, 2));
        assert_eq_float!(6.0, image.area(1, 2, 2, 3));
        assert_eq_float!(13.0, image.area(4, 0, 5, 1));
        assert_eq_float!(14.0, image.area(4, 1, 5, 2));
        assert_eq_float!(15.0, image.area(4, 2, 5, 3));
        assert_eq_float!(
            (4.0 + 5.0 + 6.0) + (7.0 + 8.0 + 9.0) + (10.0 + 11.0 + 12.0) + (13.0 + 14.0 + 15.0),
            image.area(1, 0, 5, 3)
        );

        image.add_row(&[16, 17, 18]);

        assert_eq!(3, image.columns());
        assert_eq!(6, image.rows());

        assert_eq_float!(7.0, image.area(2, 0, 3, 1));
        assert_eq_float!(8.0, image.area(2, 1, 3, 2));
        assert_eq_float!(9.0, image.area(2, 2, 3, 3));
        assert_eq_float!(16.0, image.area(5, 0, 6, 1));
        assert_eq_float!(17.0, image.area(5, 1, 6, 2));
        assert_eq_float!(18.0, image.area(5, 2, 6, 3));
        assert_eq_float!(
            (7.0 + 8.0 + 9.0) + (10.0 + 11.0 + 12.0) + (13.0 + 14.0 + 15.0) + (16.0 + 17.0 + 18.0),
            image.area(2, 0, 6, 3)
        );
    }
}
