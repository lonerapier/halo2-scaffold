use halo2_proofs::{
    arithmetic::FieldExt,
    circuit::{Chip, Layouter, Value},
    plonk::{Advice, Column, ConstraintSystem, Error, Selector, TableColumn},
    poly::Rotation,
};
use std::marker::PhantomData;

#[derive(Debug, Clone)]
pub struct XorChipConfig<F: FieldExt, const N: usize> {
    pub table: [TableColumn; 3],
    pub selector: Selector,
    _marker: PhantomData<F>,
}

impl<F: FieldExt, const N: usize> XorChipConfig<F, N> {
    pub fn configure(
        meta: &mut ConstraintSystem<F>,
        cola: Column<Advice>,
        colb: Column<Advice>,
        colc: Column<Advice>,
    ) -> Self {
        let xor_table = [(); 3].map(|_| meta.lookup_table_column());
        let selector = meta.complex_selector();

        meta.lookup("xor_lookup", |meta| {
            let s = meta.query_selector(selector);
            let [lhs, rhs, out] =
                [cola, colb, colc].map(|column| meta.query_advice(column, Rotation::cur()));

            vec![
                (s.clone() * lhs, xor_table[0]),
                (s.clone() * rhs, xor_table[1]),
                (s.clone() * out, xor_table[2]),
            ]
        });

        Self { table: xor_table, selector, _marker: PhantomData }
    }

    pub fn load(&self, layouter: &mut impl Layouter<F>) -> Result<(), Error> {
        layouter.assign_table(
            || "xor_lookup_table",
            |mut table| {
                let mut idx = 0;
                for lhs in 0..N {
                    for rhs in 0..N {
                        table.assign_cell(
                            || format!("lhs_{lhs}"),
                            self.table[0],
                            idx,
                            || Value::known(F::from_u128(lhs as u128)),
                        )?;
                        table.assign_cell(
                            || format!("rhs_{rhs}"),
                            self.table[1],
                            idx,
                            || Value::known(F::from_u128(rhs as u128)),
                        )?;
                        table.assign_cell(
                            || format!("lhs^rhs_{lhs}_{lhs}"),
                            self.table[2],
                            idx,
                            || Value::known(F::from_u128((lhs as u128) ^ (rhs as u128))),
                        )?;
                        idx += 1;
                    }
                }
                Ok(())
            },
        )
    }
}
