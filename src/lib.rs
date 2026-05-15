#![allow(non_snake_case)]

pub fn ProjectName() -> &'static str {
    "Dunewyrm"
}

#[cfg(test)]
mod tests {
    use super::ProjectName;

    #[test]
    fn ProjectNameMatches() {
        assert_eq!(ProjectName(), "Dunewyrm");
    }
}
