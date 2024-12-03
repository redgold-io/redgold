
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

pub fn format_dollar_amount_with_prefix(amount: f64) -> String {
    format!("${}", format_dollar_amount(amount))
}

pub fn format_dollar_amount_with_prefix_and_suffix(amount: f64) -> String {
    format!("${} USD", format_dollar_amount(amount))
}
