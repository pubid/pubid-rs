//! The IEEE document-code split shared by the nesc and ire flavors.
//!
//! Both flavors carry their designation through `Pubid::Ieee::
//! Components::Code`, which parses a code string into prefix / number /
//! parts / original separator and hoists the flat columns onto the
//! identifier via the CodeNumber mixin. This is that parse and its two
//! print faces (`to_s` keeps the separators each part was printed
//! with; `canonical_to_s` renders one uniform separator — the face the
//! URN reads).

/// The parsed code components.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct IeeeCode {
    /// The letter series designator (`C` of `C2`), when present.
    pub prefix: Option<String>,
    /// The main number.
    pub number: String,
    /// The dotted/dashed parts after the number.
    pub parts: Vec<String>,
    /// The separator before the parts (`.` or `-`), when parts exist.
    pub separator: Option<String>,
    /// The separators AS PRINTED, one per part boundary (a code may mix
    /// them: `7.4-3-2`).
    pub per_part: Option<Vec<String>>,
}

impl IeeeCode {
    /// Parse a code string (`Pubid::Ieee::Components::Code.parse`).
    pub fn parse(code: &str) -> Option<Self> {
        if code.is_empty() {
            return None;
        }
        // A single leading uppercase letter is the prefix.
        let prefix: Option<String> = code
            .chars()
            .next()
            .filter(|c| c.is_ascii_uppercase())
            .map(|c| c.to_string());
        let remainder = match &prefix {
            Some(prefix) => &code[prefix.len()..],
            None => code,
        };

        let separator = if remainder.contains('.') {
            Some(".")
        } else if remainder.contains('-') {
            Some("-")
        } else {
            None
        };

        let mut components = remainder
            .split(['.', '-'])
            .map(str::to_string)
            .collect::<Vec<_>>();
        let number = if components.is_empty() {
            String::new()
        } else {
            components.remove(0)
        };
        let separators = remainder
            .chars()
            .filter(|c| *c == '.' || *c == '-')
            .map(|c| c.to_string())
            .collect::<Vec<_>>();
        let per_part = if separators.len() == components.len() && !separators.is_empty() {
            Some(separators)
        } else {
            None
        };

        Some(Self {
            prefix,
            number,
            parts: components,
            separator: separator.map(str::to_string),
            per_part,
        })
    }

    /// The printed face: keeps the separator each part was printed with.
    pub fn to_s(&self) -> String {
        self.render(true)
    }

    /// The canonical face: one uniform separator across every part
    /// boundary (the one the source printed).
    pub fn canonical_to_s(&self) -> String {
        self.render(false)
    }

    fn render(&self, per_part: bool) -> String {
        let mut result = String::new();
        if let Some(prefix) = &self.prefix {
            result.push_str(prefix);
        }
        result.push_str(&self.number);
        if self.parts.is_empty() {
            return result;
        }
        let uniform = self.separator.as_deref().unwrap_or(".");
        for (index, part) in self.parts.iter().enumerate() {
            let separator = match (per_part, &self.per_part) {
                (true, Some(separators)) => {
                    separators.get(index).map(String::as_str).unwrap_or(uniform)
                }
                _ => uniform,
            };
            result.push_str(separator);
            result.push_str(part);
        }
        result
    }
}

#[cfg(test)]
mod tests {
    use super::IeeeCode;

    #[test]
    fn splits_prefix_number_and_dotted_parts() {
        let code = IeeeCode::parse("C2").expect("parse");
        assert_eq!(code.prefix.as_deref(), Some("C"));
        assert_eq!(code.number, "2");
        assert!(code.parts.is_empty());
        assert_eq!(code.canonical_to_s(), "C2");

        let dotted = IeeeCode::parse("7.S2").expect("parse");
        assert_eq!(dotted.prefix, None);
        assert_eq!(dotted.number, "7");
        assert_eq!(dotted.parts, vec!["S2".to_string()]);
        assert_eq!(dotted.separator.as_deref(), Some("."));
        assert_eq!(dotted.to_s(), "7.S2");
        assert_eq!(dotted.canonical_to_s(), "7.S2");
    }

    #[test]
    fn keeps_mixed_per_part_separators_on_the_printed_face() {
        let mixed = IeeeCode::parse("7.4-3-2").expect("parse");
        assert_eq!(mixed.number, "7");
        assert_eq!(mixed.parts, vec!["4", "3", "2"]);
        assert_eq!(mixed.separator.as_deref(), Some("."));
        assert_eq!(mixed.to_s(), "7.4-3-2");
        assert_eq!(mixed.canonical_to_s(), "7.4.3.2");
    }
}
