use sqlex_common::types::Cardinality;

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
    pub(crate) min: MinRows,
    pub(crate) max: MaxRows,
}

impl CardInterval {
    pub(crate) fn to_cardinality(self) -> Cardinality {
        match (self.min, self.max) {
            (MinRows::Zero, MaxRows::Zero) => Cardinality::ExactlyZero,
            (MinRows::One, MaxRows::One) => Cardinality::ExactlyOne,
            (MinRows::Zero, MaxRows::One) => Cardinality::AtMostOne,
            (MinRows::One, MaxRows::Many) => Cardinality::OneOrMore,
            (MinRows::Zero, MaxRows::Many) => Cardinality::ZeroOrMore,
            (MinRows::One, MaxRows::Zero) => Cardinality::ExactlyZero,
        }
    }
}
