use sqlex_common::{dialect::Dialect, types::DataType};

use crate::infer::{
    Inferencer,
    model::metadata::{InferColumn, InferMetadata},
};

impl<'a> Inferencer<'a> {
    pub(in crate::infer) fn narrow_int_literals_at_boundary(
        &self,
        metadata: InferMetadata,
    ) -> InferMetadata {
        if self.dialect != Dialect::MySQL {
            return metadata;
        }

        let columns = metadata
            .columns
            .into_iter()
            .map(|col| {
                let data_type = match (&col.data_type, &col.int_literal_info) {
                    (DataType::BigInt, Some(h)) if h.display_width <= 8 => DataType::Int,
                    (DataType::UnsignedBigInt, Some(h)) if h.display_width <= 8 => {
                        DataType::UnsignedInt
                    }
                    _ => col.data_type.clone(),
                };

                InferColumn {
                    slot_id: col.slot_id,
                    name: col.name,
                    data_type,
                    nullable: col.nullable,
                    origin: col.origin,
                    int_literal_info: None,
                }
            })
            .collect();

        InferMetadata {
            columns,
            cardinality: metadata.cardinality,
            keys: metadata.keys,
        }
    }
}
