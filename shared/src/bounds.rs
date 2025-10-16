//! Safe bounds checking and arithmetic
//! Prevents overflow, underflow, and out-of-bounds access

/// Safe multiplication with overflow checking
pub fn safe_mul(a: u64, b: u64) -> Result<u64, &'static str> {
    a.checked_mul(b).ok_or("MultiplicationOverflow")
}

/// Safe addition with overflow checking
pub fn safe_add(a: u64, b: u64) -> Result<u64, &'static str> {
    a.checked_add(b).ok_or("AdditionOverflow")
}

/// Safe subtraction with underflow checking
pub fn safe_sub(a: u64, b: u64) -> Result<u64, &'static str> {
    a.checked_sub(b).ok_or("SubtractionUnderflow")
}

/// Safe division with zero checking
pub fn safe_div(a: u64, b: u64) -> Result<u64, &'static str> {
    if b == 0 {
        return Err("DivisionByZero");
    }
    Ok(a / b)
}

/// Check if index is within bounds
pub fn check_bounds(index: usize, length: usize) -> Result<(), &'static str> {
    if index >= length {
        return Err("IndexOutOfBounds");
    }
    Ok(())
}

/// Check if a range is valid within bounds
pub fn check_range(start: usize, end: usize, length: usize) -> Result<(), &'static str> {
    if start > end {
        return Err("InvalidRange");
    }
    if end > length {
        return Err("RangeOutOfBounds");
    }
    Ok(())
}

/// Calculate percentage safely (result in basis points, 10000 = 100%)
pub fn safe_percentage(amount: u64, percentage_bps: u64) -> Result<u64, &'static str> {
    if percentage_bps > 10000 {
        return Err("InvalidPercentage");
    }
    let result = safe_mul(amount, percentage_bps)?;
    safe_div(result, 10000)
}

/// Check if value is within min/max range
pub fn check_value_range(value: u64, min: u64, max: u64) -> Result<(), &'static str> {
    if value < min {
        return Err("ValueBelowMinimum");
    }
    if value > max {
        return Err("ValueAboveMaximum");
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_safe_mul() {
        assert_eq!(safe_mul(10, 20).unwrap(), 200);
        assert!(safe_mul(u64::MAX, 2).is_err());
    }

    #[test]
    fn test_safe_add() {
        assert_eq!(safe_add(10, 20).unwrap(), 30);
        assert!(safe_add(u64::MAX, 1).is_err());
    }

    #[test]
    fn test_bounds() {
        assert!(check_bounds(5, 10).is_ok());
        assert!(check_bounds(10, 10).is_err());
        assert!(check_bounds(15, 10).is_err());
    }

    #[test]
    fn test_percentage() {
        // 10% of 1000 = 100
        assert_eq!(safe_percentage(1000, 1000).unwrap(), 100);
        // 50% of 1000 = 500
        assert_eq!(safe_percentage(1000, 5000).unwrap(), 500);
        // Invalid percentage
        assert!(safe_percentage(1000, 10001).is_err());
    }
}
