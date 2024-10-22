pub fn gradient(mut iter: impl Iterator<Item=f64>, output: &mut Vec<f64>) {
    if let Some(mut f0) = iter.next() {
        if let Some(mut f1) = iter.next() {
            output.push(f1 - f0);
            if let Some(mut f2) = iter.next() {
                output.push((f2 - f0) / 2.0);
                for next in iter {
                    f0 = f1;
                    f1 = f2;
                    f2 = next;
                    output.push((f2 - f0) / 2.0);
                }
                output.push(f2 - f1);
            } else {
                output.push(f1 - f0);
            }
        } else {
            output.push(0.0);
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::assert_eq_float;
    use crate::gradient::gradient;

    #[test]
    fn empty() {
        let input = [];
        let mut output = Vec::new();
        gradient(input.into_iter(), &mut output);
        assert_eq!(0, output.len());
    }

    #[test]
    fn one_element() {
        let mut output = Vec::new();
        let input = [1.0];
        gradient(input.into_iter(), &mut output);
        assert_eq!(1, output.len());
        assert_eq_float!(0.0, output[0]);
    }

    #[test]
    fn two_elements() {
        let mut output = Vec::new();
        let input = [1.0, 2.0];
        gradient(input.into_iter(), &mut output);
        assert_eq!(2, output.len());
        assert_eq_float!(1.0, output[0]);
        assert_eq_float!(1.0, output[1]);
    }

    #[test]
    fn three_elements() {
        let mut output = Vec::new();
        let input = [1.0, 2.0, 4.0];
        gradient(input.into_iter(), &mut output);
        assert_eq!(3, output.len());
        assert_eq_float!(1.0, output[0]);
        assert_eq_float!(1.5, output[1]);
        assert_eq_float!(2.0, output[2]);
    }

    #[test]
    fn four_elements() {
        let mut output = Vec::new();
        let input = [1.0, 2.0, 4.0, 10.0];
        gradient(input.into_iter(), &mut output);
        assert_eq!(4, output.len());
        assert_eq_float!(1.0, output[0]);
        assert_eq_float!(1.5, output[1]);
        assert_eq_float!(4.0, output[2]);
        assert_eq_float!(6.0, output[3]);
    }
}