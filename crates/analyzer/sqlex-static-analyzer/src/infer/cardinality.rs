use sqlex_common::types::Cardinality;

use crate::diagnostics::{Diagnostic, Phase};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum MinRows {
    Zero,
    One,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum MaxRows {
    Zero,
    One,
    Many,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct CardInterval {
    min: MinRows,
    max: MaxRows,
}

impl CardInterval {
    pub(crate) const fn exactly_zero() -> Self {
        Self {
            min: MinRows::Zero,
            max: MaxRows::Zero,
        }
    }

    pub(crate) fn try_new(
        min: MinRows,
        max: MaxRows,
        location: &'static str,
    ) -> Result<Self, Diagnostic> {
        if !is_valid_interval(min, max) {
            return Err(Diagnostic::new(
                "I4203",
                Phase::Infer,
                format!("invalid cardinality interval [{min:?}, {max:?}] at {location}"),
            ));
        }
        Ok(Self { min, max })
    }

    pub(crate) const fn exactly_one() -> Self {
        Self {
            min: MinRows::One,
            max: MaxRows::One,
        }
    }

    pub(crate) const fn at_most_one() -> Self {
        Self {
            min: MinRows::Zero,
            max: MaxRows::One,
        }
    }

    pub(crate) const fn one_or_more() -> Self {
        Self {
            min: MinRows::One,
            max: MaxRows::Many,
        }
    }

    pub(crate) const fn zero_or_more() -> Self {
        Self {
            min: MinRows::Zero,
            max: MaxRows::Many,
        }
    }

    pub(crate) const fn min(self) -> MinRows {
        self.min
    }

    pub(crate) const fn max(self) -> MaxRows {
        self.max
    }

    pub(crate) const fn drop_lower_bound(self) -> Self {
        Self {
            min: MinRows::Zero,
            max: self.max,
        }
    }

    pub(crate) const fn constrain_at_most_one(self) -> Self {
        let max = match self.max {
            MaxRows::Zero => MaxRows::Zero,
            MaxRows::One | MaxRows::Many => MaxRows::One,
        };
        Self { min: self.min, max }
    }

    pub(crate) fn to_cardinality(self) -> Cardinality {
        match (self.min, self.max) {
            (MinRows::Zero, MaxRows::Zero) => Cardinality::ExactlyZero,
            (MinRows::One, MaxRows::One) => Cardinality::ExactlyOne,
            (MinRows::Zero, MaxRows::One) => Cardinality::AtMostOne,
            (MinRows::One, MaxRows::Many) => Cardinality::OneOrMore,
            (MinRows::Zero, MaxRows::Many) => Cardinality::ZeroOrMore,
            (MinRows::One, MaxRows::Zero) => {
                panic!("invalid card interval should be rejected at construction")
            },
        }
    }
}

const fn is_valid_interval(min: MinRows, max: MaxRows) -> bool {
    lower_rank(min) <= upper_rank(max)
}

const fn lower_rank(min: MinRows) -> u8 {
    match min {
        MinRows::Zero => 0,
        MinRows::One => 1,
    }
}

const fn upper_rank(max: MaxRows) -> u8 {
    match max {
        MaxRows::Zero => 0,
        MaxRows::One => 1,
        MaxRows::Many => 2,
    }
}

#[cfg(test)]
mod tests {
    use sqlex_common::types::Cardinality;

    use crate::infer::cardinality::{CardInterval, MaxRows, MinRows};

    #[test]
    fn invalid_interval_is_rejected() {
        let result = CardInterval::try_new(
            MinRows::One,
            MaxRows::Zero,
            "cardinality::tests::invalid_interval_is_rejected",
        );

        let diagnostic = result.expect_err("invalid interval should return diagnostic");
        assert_eq!(diagnostic.code, "I4203");
    }

    #[test]
    fn constructors_map_to_public_cardinality() {
        assert_eq!(
            CardInterval::try_new(
                MinRows::Zero,
                MaxRows::Zero,
                "cardinality::tests::constructors_map_to_public_cardinality",
            )
            .expect("valid interval should be created")
            .to_cardinality(),
            Cardinality::ExactlyZero
        );
        assert_eq!(
            CardInterval::exactly_one().to_cardinality(),
            Cardinality::ExactlyOne
        );
        assert_eq!(
            CardInterval::at_most_one().to_cardinality(),
            Cardinality::AtMostOne
        );
        assert_eq!(
            CardInterval::one_or_more().to_cardinality(),
            Cardinality::OneOrMore
        );
        assert_eq!(
            CardInterval::zero_or_more().to_cardinality(),
            Cardinality::ZeroOrMore
        );
    }
}
