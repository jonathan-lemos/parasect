#[allow(unused)]
pub fn truncate_elipsis(s: &str, width: usize) -> String {
    if s.len() <= width {
        s.to_string()
    } else if width == 0 {
        "".into()
    } else {
        let mut trunc = s[..width - 1].to_string();
        trunc.push('…');
        trunc
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_truncate_elipsis() {
        assert_eq!(truncate_elipsis("amogus", 0), "");
        assert_eq!(truncate_elipsis("amogus", 1), "…");
        assert_eq!(truncate_elipsis("amogus", 5), "amog…");
        assert_eq!(truncate_elipsis("amogus", 6), "amogus");
    }
}
