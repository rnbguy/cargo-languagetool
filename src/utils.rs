pub const fn decimal_places(mut num: usize) -> usize {
    let mut places = 0;

    loop {
        num /= 10;
        places += 1;
        if num == 0 {
            break;
        }
    }

    places
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_decimal_places() {
        for i in 0..1_000_000 {
            assert_eq!(decimal_places(i), i.to_string().len());
        }
    }
}
