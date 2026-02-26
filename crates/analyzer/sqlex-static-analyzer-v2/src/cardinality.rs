use sqlex_common::types::Cardinality;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MinRows {
    Zero,
    One,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MaxRows {
    Zero,
    One,
    Many,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CardInterval {
    pub min: MinRows,
    pub max: MaxRows,
}

impl CardInterval {
    pub const fn exactly_zero() -> Self {
        Self {
            min: MinRows::Zero,
            max: MaxRows::Zero,
        }
    }

    pub const fn exactly_one() -> Self {
        Self {
            min: MinRows::One,
            max: MaxRows::One,
        }
    }

    pub const fn at_most_one() -> Self {
        Self {
            min: MinRows::Zero,
            max: MaxRows::One,
        }
    }

    pub const fn one_or_more() -> Self {
        Self {
            min: MinRows::One,
            max: MaxRows::Many,
        }
    }

    pub const fn zero_or_more() -> Self {
        Self {
            min: MinRows::Zero,
            max: MaxRows::Many,
        }
    }

    pub const fn drop_lower_bound(self) -> Self {
        Self {
            min: MinRows::Zero,
            max: self.max,
        }
    }

    pub const fn constrain_at_most_one(self) -> Self {
        let max = match self.max {
            MaxRows::Zero => MaxRows::Zero,
            MaxRows::One | MaxRows::Many => MaxRows::One,
        };
        Self { min: self.min, max }
    }

    pub fn to_cardinality(self) -> Cardinality {
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
