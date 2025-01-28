use std::fmt::Write;

pub fn format_dollar_amount(amount: f64) -> String {
    let mut result = String::new();

    // Format the integer part
    let int_part = amount.abs().trunc() as u64;
    let int_formatted = format!("{}", int_part)
        .chars()
        .rev()
        .enumerate()
        .fold(String::new(), |mut acc, (i, c)| {
            if i > 0 && i % 3 == 0 {
                acc.push(',');
            }
            acc.push(c);
            acc
        })
        .chars()
        .rev()
        .collect::<String>();

    // Format the fractional part
    let frac_part = (amount.abs().fract() * 100.0).round() as u8;

    // Combine the parts
    write!(result, "{}", if amount < 0.0 { "-" } else { "" }).unwrap();
    write!(result, "{}.{:02}", int_formatted, frac_part).unwrap();

    result
}

pub fn format_dollar_amount_brief(amount: f64) -> String {
    let abs_amount = amount.abs();
    let mut result = String::new();

    // Handle different magnitude ranges
    let (divided_amount, suffix) = if abs_amount >= 1_000_000_000.0 {
        (abs_amount / 1_000_000_000.0, "b")
    } else if abs_amount >= 1_000_000.0 {
        (abs_amount / 1_000_000.0, "m")
    } else if abs_amount >= 1_000.0 {
        (abs_amount / 1_000.0, "k")
    } else {
        (abs_amount, "")
    };

    // Format the integer part
    let int_part = divided_amount.trunc() as u64;
    let int_formatted = if suffix.is_empty() {
        // Use comma formatting for full numbers
        format!("{}", int_part)
            .chars()
            .rev()
            .enumerate()
            .fold(String::new(), |mut acc, (i, c)| {
                if i > 0 && i % 3 == 0 {
                    acc.push(',');
                }
                acc.push(c);
                acc
            })
            .chars()
            .rev()
            .collect::<String>()
    } else {
        // Don't use commas for abbreviated numbers
        int_part.to_string()
    };

    // Format the fractional part
    let frac_part = (divided_amount.fract() * 100.0).round() as u8;

    // Combine the parts
    write!(result, "{}", if amount < 0.0 { "-" } else { "" }).unwrap();
    write!(result, "{}.{:02}{}", int_formatted, frac_part, suffix).unwrap();

    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_dollar_amount() {
        assert_eq!(format_dollar_amount_brief(1234567.89), "1.23m");
        assert_eq!(format_dollar_amount_brief(1234.56), "1.23k");
        assert_eq!(format_dollar_amount_brief(123.45), "123.45");
        assert_eq!(format_dollar_amount_brief(-1234567.89), "-1.23m");
        assert_eq!(format_dollar_amount_brief(1_234_567_890.12), "1.23b");
    }
}

pub fn format_dollar_amount_with_prefix(amount: f64) -> String {
    format!("${}", format_dollar_amount(amount))
}

pub fn format_dollar_amount_with_prefix_and_suffix(amount: f64) -> String {
    format!("${} USD", format_dollar_amount(amount))
}

pub fn format_dollar_amount_brief_with_prefix_and_suffix(amount: f64) -> String {
    format!("${} USD", format_dollar_amount_brief(amount))
}
