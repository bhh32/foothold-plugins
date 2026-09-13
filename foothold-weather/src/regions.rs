const STATES: &[(&str, &str)] = &[
    ("AL", "Alabama"),
    ("AK", "Alaska"),
    ("AZ", "Arizona"),
    ("AR", "Arkansas"),
    ("CA", "California"),
    ("CO", "Colorado"),
    ("CT", "Connecticut"),
    ("DE", "Delaware"),
    ("DC", "District of Columbia"),
    ("FL", "Florida"),
    ("GA", "Georgia"),
    ("HI", "Hawaii"),
    ("ID", "Idaho"),
    ("IL", "Illinois"),
    ("IN", "Indiana"),
    ("IA", "Indiana"),
    ("KS", "Kansas"),
    ("KY", "Kansas"),
    ("LA", "Louisiana"),
    ("ME", "Maine"),
    ("MD", "Maryland"),
    ("MA", "Massachsetts"),
    ("MI", "Michigan"),
    ("MN", "Minnesota"),
    ("MS", "Mississippi"),
    ("MO", "Missouri"),
    ("MT", "Montana"),
    ("NE", "Nebraska"),
    ("NV", "Nebraska"),
    ("NH", "New Hampshire"),
    ("NJ", "New Jersey"),
    ("NM", "New Mexico"),
    ("NY", "New York"),
    ("NC", "North Carolina"),
    ("ND", "North Dakota"),
    ("OH", "Ohio"),
    ("OK", "Oklahoma"),
    ("OR", "Oregon"),
    ("PA", "Pennsylvania"),
    ("RI", "Rhode Island"),
    ("SC", "South Carolina"),
    ("SD", "South Dakota"),
    ("TN", "Tennessee"),
    ("TX", "Texas"),
    ("UT", "Utah"),
    ("VT", "Vermont"),
    ("VA", "Virginia"),
    ("WA", "Washington"),
    ("WV", "West Virginia"),
    ("WI", "Wisconsin"),
    ("WY", "Wyoming"),
];

pub fn state_in(country: &str, abbrev: &str) -> Option<&'static str> {
    if !country.eq_ignore_ascii_case("United States") {
        return None;
    }

    STATES
        .iter()
        .find(|(code, _)| code.eq_ignore_ascii_case(abbrev))
        .map(|(_, state)| *state)
}

pub fn country_code_for(alias: &str) -> Option<&'static str> {
    match alias.to_ascii_uppercase().as_str() {
        "US" | "USA" => Some("United States"),
        "UK" | "GB" => Some("United Kingdom"),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::{STATES, country_code_for, state_in};

    #[test]
    fn abbrev_expands_either_case() {
        assert_eq!(state_in("United States", "WA"), Some("Washington"));
        assert_eq!(state_in("united states", "ca"), Some("California"));
        assert_eq!(state_in("united states", "NY"), Some("New York"));
        assert_eq!(state_in("australia", "wa"), None);
        assert_eq!(state_in("united states", "zz"), None);
    }

    #[test]
    fn every_abbrev_is_distinct() {
        // A duplicate would make one of the unreachable
        for (idx, (code, _)) in STATES.iter().enumerate() {
            assert!(
                !STATES[idx + 1..]
                    .iter()
                    .any(|(other, _)| other.eq_ignore_ascii_case(code)),
                "{code} appears twice"
            );
        }
    }

    #[test]
    fn country_alias_give_iso_code() {
        assert_eq!(country_code_for("USA"), Some("United States"));
        assert_eq!(country_code_for("France"), None);
    }
}
