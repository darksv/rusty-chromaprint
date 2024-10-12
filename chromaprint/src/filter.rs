#[derive(Debug, Clone, Copy)]
pub struct Filter {
    kind: FilterKind,
    y: usize,
    height: usize,
    width: usize,
}

#[derive(Debug, Clone, Copy)]
pub enum FilterKind {
    Filter0,
    Filter1,
    Filter2,
    Filter3,
    Filter4,
    Filter5,
}

impl Filter {
    pub(crate) const fn new(kind: FilterKind, y: usize, height: usize, width: usize) -> Self {
        Self { kind, y, height, width }
    }

    pub(crate) fn apply(&self, image: &impl Image, x: usize) -> f64 {
        let filter = match self.kind {
            FilterKind::Filter0 => filter0,
            FilterKind::Filter1 => filter1,
            FilterKind::Filter2 => filter2,
            FilterKind::Filter3 => filter3,
            FilterKind::Filter4 => filter4,
            FilterKind::Filter5 => filter5,
        };
        return filter(image, x, self.y, self.width, self.height, subtract_log);
    }

    pub(crate) fn width(&self) -> usize {
        self.width
    }
}

fn subtract_log(a: f64, b: f64) -> f64 {
    let r = f64::ln((1.0 + a) / (1.0 + b));
    assert!(!r.is_nan());
    return r;
}

pub trait Image {
    fn area(&self, x: usize, y: usize, w: usize, h: usize) -> f64;
}

type Comparator = fn(f64, f64) -> f64;

// oooooooooooooooo
// oooooooooooooooo
// oooooooooooooooo
// oooooooooooooooo
fn filter0(image: &impl Image, x: usize, y: usize, w: usize, h: usize, cmp: Comparator) -> f64 {
    assert!(w >= 1);
    assert!(h >= 1);

    let a = image.area(x, y, x + w, y + h);
    let b = 0.0;

    return cmp(a, b);
}

// ................
// ................
// oooooooooooooooo
// oooooooooooooooo
fn filter1(image: &impl Image, x: usize, y: usize, w: usize, h: usize, cmp: Comparator) -> f64 {
    assert!(w >= 1);
    assert!(h >= 1);

    let h_2 = h / 2;

    let a = image.area(x, y + h_2, x + w, y + h);
    let b = image.area(x, y, x + w, y + h_2);

    return cmp(a, b);
}


// .......ooooooooo
// .......ooooooooo
// .......ooooooooo
// .......ooooooooo
fn filter2(image: &impl Image, x: usize, y: usize, w: usize, h: usize, cmp: Comparator) -> f64 {
    assert!(w >= 1);
    assert!(h >= 1);

    let w_2 = w / 2;

    let a = image.area(x + w_2, y, x + w, y + h);
    let b = image.area(x, y, x + w_2, y + h);

    return cmp(a, b);
}

// .......ooooooooo
// .......ooooooooo
// ooooooo.........
// ooooooo.........
fn filter3(image: &impl Image, x: usize, y: usize, w: usize, h: usize, cmp: Comparator) -> f64 {
    assert!(w >= 1);
    assert!(h >= 1);

    let w_2 = w / 2;
    let h_2 = h / 2;

    let a = image.area(x, y + h_2, x + w_2, y + h) +
        image.area(x + w_2, y, x + w, y + h_2);
    let b = image.area(x, y, x + w_2, y + h_2) +
        image.area(x + w_2, y + h_2, x + w, y + h);

    return cmp(a, b);
}

// ................
// oooooooooooooooo
// ................
fn filter4(image: &impl Image, x: usize, y: usize, w: usize, h: usize, cmp: Comparator) -> f64 {
    assert!(w >= 1);
    assert!(h >= 1);

    let h_3 = h / 3;

    let a = image.area(x, y + h_3, x + w, y + 2 * h_3);
    let b = image.area(x, y, x + w, y + h_3) +
        image.area(x, y + 2 * h_3, x + w, y + h);

    return cmp(a, b);
}

// .....oooooo.....
// .....oooooo.....
// .....oooooo.....
// .....oooooo.....
fn filter5(image: &impl Image, x: usize, y: usize, w: usize, h: usize, cmp: Comparator) -> f64 {
    assert!(w >= 1);
    assert!(h >= 1);

    let w_3 = w / 3;

    let a = image.area(x + w_3, y, x + 2 * w_3, y + h);
    let b = image.area(x, y, x + w_3, y + h) +
        image.area(x + 2 * w_3, y, x + w, y + h);

    return cmp(a, b);
}


#[cfg(test)]
mod tests {
    use crate::assert_eq_float;
    use crate::filter::{Filter, filter0, filter1, filter2, filter3, filter4, filter5, FilterKind, subtract_log};
    use crate::rolling_image::RollingIntegralImage;

    #[test]
    fn test_compare_subtract() {
        let res = subtract(2.0, 1.0);
        assert_eq_float!(1.0, res);
    }

    #[test]
    fn test_compare_subtract_log() {
        let res = subtract_log(2.0, 1.0);
        assert_eq_float!(0.4054651, res);
    }

    #[test]
    fn test_filter_with_filter0() {
        let data = [
            0.0, 1.0,
            2.0, 3.0,
        ];
        let mut integral_image = RollingIntegralImage::from_data(2, &data);
        let flt1 = Filter::new(FilterKind::Filter0, 0, 1, 1);
        assert_eq_float!(0.0, flt1.apply(&mut integral_image, 0));
        assert_eq_float!(1.0986123, flt1.apply(&mut integral_image, 1));
    }

    #[test]
    fn test_filter0() {
        let data = [
            1.0, 2.0, 3.0,
            4.0, 5.0, 6.0,
            7.0, 8.0, 9.0,
        ];

        let integral_image = RollingIntegralImage::from_data(3, &data);

        let res = filter0(&integral_image, 0, 0, 1, 1, subtract);
        assert_eq_float!(1.0, res);
        let res = filter0(&integral_image, 0, 0, 2, 2, subtract);
        assert_eq_float!(12.0, res);
        let res = filter0(&integral_image, 0, 0, 3, 3, subtract);
        assert_eq_float!(45.0, res);
        let res = filter0(&integral_image, 1, 1, 2, 2, subtract);
        assert_eq_float!(28.0, res);
        let res = filter0(&integral_image, 2, 2, 1, 1, subtract);
        assert_eq_float!(9.0, res);
        let res = filter0(&integral_image, 0, 0, 3, 1, subtract);
        assert_eq_float!(12.0, res);
        let res = filter0(&integral_image, 0, 0, 1, 3, subtract);
        assert_eq_float!(6.0, res);
    }

    #[test]
    fn test_filter1() {
        let data = [
            1.0, 2.1, 3.4,
            3.1, 4.1, 5.1,
            6.0, 7.1, 8.0,
        ];

        let integral_image = RollingIntegralImage::from_data(3, &data);
        let res = filter1(&integral_image, 0, 0, 1, 1, subtract);
        assert_eq_float!(1.0 - 0.0, res);
        let res = filter1(&integral_image, 1, 1, 1, 1, subtract);
        assert_eq_float!(4.1 - 0.0, res);
        let res = filter1(&integral_image, 0, 0, 1, 2, subtract);
        assert_eq_float!(2.1 - 1.0, res);
        let res = filter1(&integral_image, 0, 0, 2, 2, subtract);
        assert_eq_float!((2.1 + 4.1) - (1.0 + 3.1), res);
        let res = filter1(&integral_image, 0, 0, 3, 2, subtract);
        assert_eq_float!((2.1 + 4.1 + 7.1) - (1.0 + 3.1 + 6.0), res);
    }

    #[test]
    fn test_filter2() {
        let data = [
            1.0, 2.0, 3.0,
            3.0, 4.0, 5.0,
            6.0, 7.0, 8.0,
        ];

        let integral_image = RollingIntegralImage::from_data(3, &data);
        let res = filter2(&integral_image, 0, 0, 2, 1, subtract);
        assert_eq_float!(2.0, res); // 3 - 1
        let res = filter2(&integral_image, 0, 0, 2, 2, subtract);
        assert_eq_float!(4.0, res); // 3+4 - 1+2
        let res = filter2(&integral_image, 0, 0, 2, 3, subtract);
        assert_eq_float!(6.0, res); // 3+4+5 - 1+2+3
    }

    #[test]
    fn test_filter3() {
        let data = [
            1.0, 2.1, 3.4,
            3.1, 4.1, 5.1,
            6.0, 7.1, 8.0,
        ];

        let integral_image = RollingIntegralImage::from_data(3, &data);
        let res = filter3(&integral_image, 0, 0, 2, 2, subtract);
        assert_eq_float!(0.1, res); // 2.1+3.1 - 1+4.1
        let res = filter3(&integral_image, 1, 1, 2, 2, subtract);
        assert_eq_float!(0.1, res); // 4+8 - 5+7
        let res = filter3(&integral_image, 0, 1, 2, 2, subtract);
        assert_eq_float!(0.3, res); // 2.1+5.1 - 3.4+4.1
    }

    #[test]
    fn test_filter4() {
        let data = [
            1.0, 2.0, 3.0,
            3.0, 4.0, 5.0,
            6.0, 7.0, 8.0,
        ];

        let integral_image = RollingIntegralImage::from_data(3, &data);
        let res = filter4(&integral_image, 0, 0, 3, 3, subtract);
        assert_eq_float!(-13.0, res); // 2+4+7 - (1+3+6) - (3+5+8)
    }

    #[test]
    fn test_filter5() {
        let data = [
            1.0, 2.0, 3.0,
            3.0, 4.0, 5.0,
            6.0, 7.0, 8.0,
        ];

        let integral_image = RollingIntegralImage::from_data(3, &data);
        let res = filter5(&integral_image, 0, 0, 3, 3, subtract);
        assert_eq_float!(-15.0, res); // 3+4+5 - (1+2+3) - (6+7+8)
    }

    fn subtract(a: f64, b: f64) -> f64 {
        return a - b;
    }
}
