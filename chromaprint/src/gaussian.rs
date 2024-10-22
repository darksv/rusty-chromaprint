pub fn gaussian_filter<'a>(mut input: &'a mut [f64], mut output: &'a mut [f64], sigma: f64, n: usize) {
    let w = f64::sqrt(12.0 * sigma * sigma / n as f64 + 1.0).floor() as usize;
    let wl = w - (w % 2 == 0) as usize;
    let wu = wl + 2;

    let fwl = wl as f64;
    let m = ((12.0 * sigma * sigma - n as f64 * fwl * fwl - 4.0 * n as f64 * fwl - 3.0 * n as f64) / (-4.0 * fwl - 4.0)).round() as usize;

    let mut data1 = &mut input;
    let mut data2 = &mut output;

    for _ in 0..m {
        box_filter(data1, data2, wl);
        std::mem::swap(&mut data1, &mut data2);
    }

    for _ in m..n {
        box_filter(data1, data2, wu);
        std::mem::swap(&mut data1, &mut data2);
    }

    if data1.as_ptr() != output.as_ptr() {
        output.copy_from_slice(input);
    }
}


fn box_filter(input: &[f64], output: &mut [f64], w: usize) {
    let size = input.len();
    if w == 0 || size == 0 {
        return;
    }

    let wl = w / 2;
    let wr = w - wl;

    let mut it1 = ReflectIterator::new(size);
    let mut it2 = ReflectIterator::new(size);

    for _ in 0..wl {
        it1.move_back();
        it2.move_back();
    }

    let mut sum = 0.0;
    for _ in 0..w {
        sum += input[it2.pos];
        it2.move_forward();
    }

    let mut out = SliceWriter::new(output);

    if size > w {
        for _ in 0..wl {
            out.push(sum / w as f64);
            sum += input[it2.pos] - input[it1.pos];
            it1.move_forward();
            it2.move_forward();
        }
        for _ in 0..size - w - 1 {
            out.push(sum / w as f64);
            sum += input[it2.pos] - input[it1.pos];
            it2.pos += 1;
            it1.pos += 1;
        }
        for _ in 0..wr + 1 {
            out.push(sum / w as f64);
            sum += input[it2.pos] - input[it1.pos];
            it1.move_forward();
            it2.move_forward();
        }
    } else {
        for _ in 0..size {
            out.push(sum / w as f64);
            sum += input[it2.pos] - input[it1.pos];
            it1.move_forward();
            it2.move_forward();
        }
    }
}


struct SliceWriter<'a, T> {
    slice: &'a mut [T],
    index: usize,
}


impl<'a, T> SliceWriter<'a, T> {
    fn new(slice: &'a mut [T]) -> Self {
        Self {
            slice,
            index: 0,
        }
    }

    fn push(&mut self, value: T) {
        self.slice[self.index] = value;
        self.index += 1;
    }
}

struct ReflectIterator {
    size: usize,
    pos: usize,
    forward: bool,
}

impl ReflectIterator {
    fn new(size: usize) -> Self {
        Self {
            size,
            pos: 0,
            forward: true,
        }
    }

    fn move_forward(&mut self) {
        if self.forward {
            if self.pos + 1 == self.size {
                self.forward = false;
            } else {
                self.pos += 1;
            }
        } else if self.pos == 0 {
            self.forward = true;
        } else {
            self.pos -= 1;
        }
    }

    fn move_back(&mut self) {
        if self.forward {
            if self.pos == 0 {
                self.forward = false;
            } else {
                self.pos -= 1;
            }
        } else if self.pos + 1 == self.size {
            self.forward = true;
        } else {
            self.pos += 1;
        }
    }

    #[cfg(test)]
    fn safe_forward_distance(&mut self) -> usize {
        if self.forward {
            return self.size - self.pos - 1;
        }
        return 0;
    }
}

#[cfg(test)]
mod tests {
    use crate::assert_eq_float;
    use crate::gaussian::{box_filter, gaussian_filter, ReflectIterator};

    #[test]
    fn reflect_iterator() {
        let data = [1, 2, 3, 4, 5, 6, 7, 8, 9];
        let mut it = ReflectIterator::new(data.len());
        for _ in 0..3 {
            it.move_back();
        }
        assert_eq!(3, data[it.pos]);
        assert_eq!(0, it.safe_forward_distance());
        it.move_forward();
        assert_eq!(2, data[it.pos]);
        assert_eq!(0, it.safe_forward_distance());
        it.move_forward();
        assert_eq!(1, data[it.pos]);
        assert_eq!(0, it.safe_forward_distance());
        it.move_forward();
        assert_eq!(1, data[it.pos]);
        assert_eq!(8, it.safe_forward_distance());
        it.move_forward();
        assert_eq!(2, data[it.pos]);
    }

    #[test]
    fn width1() {
        let input = [1.0, 2.0, 4.0];
        let mut output = input.clone();
        box_filter(&input, &mut output, 1);
        assert_eq!(input.len(), output.len());
        assert_eq_float!(1.0, output[0]);
        assert_eq_float!(2.0, output[1]);
        assert_eq_float!(4.0, output[2]);
    }


    #[test]
    fn width2() {
        let input = [1.0, 2.0, 4.0];
        let mut output = input.clone();
        box_filter(&input, &mut output, 2);
        assert_eq!(input.len(), output.len());
        assert_eq_float!(1.0, output[0]);
        assert_eq_float!(1.5, output[1]);
        assert_eq_float!(3.0, output[2]);
    }

    #[test]
    fn width3() {
        let input = [1.0, 2.0, 4.0];
        let mut output = input.clone();
        box_filter(&input, &mut output, 3);
        assert_eq!(input.len(), output.len());
        assert_eq_float!(1.333333333, output[0]);
        assert_eq_float!(2.333333333, output[1]);
        assert_eq_float!(3.333333333, output[2]);
    }

    #[test]
    fn width4() {
        let input = [1.0, 2.0, 4.0];
        let mut output = input.clone();
        box_filter(&input, &mut output, 4);
        assert_eq!(input.len(), output.len());
        assert_eq_float!(1.5, output[0]);
        assert_eq_float!(2.0, output[1]);
        assert_eq_float!(2.75, output[2]);
    }

    #[test]
    fn width5() {
        let input = [1.0, 2.0, 4.0];
        let mut output = input.clone();
        box_filter(&input, &mut output, 5);
        assert_eq!(input.len(), output.len());
        assert_eq_float!(2.0, output[0]);
        assert_eq_float!(2.4, output[1]);
        assert_eq_float!(2.6, output[2]);
    }

    #[test]
    fn gaussian1() {
        let mut input = [1.0, 2.0, 4.0];
        let mut output = input.clone();
        gaussian_filter(&mut input, &mut output, 1.6, 3);
        assert_eq!(input.len(), output.len());
        assert_eq_float!(1.88888889, output[0]);
        assert_eq_float!(2.33333333, output[1]);
        assert_eq_float!(2.77777778, output[2]);
    }

    #[test]
    fn gaussian2() {
        let mut input = [1.0, 2.0, 4.0];
        let mut output = input.clone();
        gaussian_filter(&mut input, &mut output, 3.6, 4);
        assert_eq!(input.len(), output.len());
        assert_eq_float!(2.3322449, output[0]);
        assert_eq_float!(2.33306122, output[1]);
        assert_eq_float!(2.33469388, output[2]);
    }
}