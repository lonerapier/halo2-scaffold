use halo2_proofs::circuit::{Layouter, Table, Value};
use halo2_proofs::halo2curves::FieldExt;
use halo2_proofs::plonk::{Advice, Column, ConstraintSystem, Instance, Selector, TableColumn};
use halo2_proofs::poly::Rotation;
use std::marker::PhantomData;

use super::xor_chip::XorChipConfig;

#[allow(dead_code)]
#[derive(Debug, Clone)]
pub struct FibSquaredConfig<F: FieldExt, const N: usize> {
    a: Column<Advice>,
    b: Column<Advice>,
    c: Column<Advice>,
    selector: Selector,
    instance: Column<Instance>,
    xor_table: XorChipConfig<F, N>,
    _marker: PhantomData<F>,
}

impl<F: FieldExt, const N: usize> FibSquaredConfig<F, N> {
    pub fn configure(meta: &mut ConstraintSystem<F>) -> Self {
        let [a, b, c] = [(); 3].map(|_| meta.advice_column());
        let selector = meta.selector();
        let instance = meta.instance_column();

        // let xor_selector = meta.complex_selector();
        let xor_table = XorChipConfig::configure(meta, a, b, c);

        [a, b, c].map(|column| meta.enable_equality(column));
        meta.enable_equality(instance);

        meta.create_gate("fib_squared gate", |meta| {
            let [x, y, out] = [a, b, c].map(|column| meta.query_advice(column, Rotation::cur()));
            let s = meta.query_selector(selector);

            let x_sq = x.square();
            let y_sq = y.square();

            vec![s * (x_sq + y_sq - out)]
        });

        Self { a, b, c, selector, instance, xor_table, _marker: PhantomData }
    }

    // pub fn assign_advice(&self, mut layouter: Impl Layouter<F>, a_val: Value<AssignedValue<F>>, b_val: Value<AssignedValue<F>>) -> Self {

    // }
}

#[cfg(test)]
mod tests {
    use halo2_proofs::dev::MockProver;
    use halo2_proofs::halo2curves::bn256::Fr;
    use halo2_proofs::{circuit::SimpleFloorPlanner, plonk::Circuit};

    use super::*;

    #[derive(Default)]
    pub struct FibSquredCircuit<F: FieldExt, const N: usize> {
        pub a: Value<F>,
        pub b: Value<F>,
    }

    impl<F: FieldExt, const N: usize> Circuit<F> for FibSquredCircuit<F, N> {
        type Config = FibSquaredConfig<F, N>;
        type FloorPlanner = SimpleFloorPlanner;

        fn without_witnesses(&self) -> Self {
            Self::default()
        }

        fn configure(meta: &mut ConstraintSystem<F>) -> Self::Config {
            Self::Config::configure(meta)
        }

        fn synthesize(
            &self,
            config: Self::Config,
            mut layouter: impl Layouter<F>,
        ) -> Result<(), halo2_proofs::plonk::Error> {
            config.xor_table.load(&mut layouter);

            let (mut b, mut c) = layouter.namespace(|| "first_row").assign_region(
                || "first row",
                |mut region| {
                    let a = region.assign_advice_from_instance(
                        || "first_row_a",
                        config.instance,
                        0,
                        config.a,
                        0,
                    )?;

                    let b = region.assign_advice_from_instance(
                        || "first_row_b",
                        config.instance,
                        1,
                        config.b,
                        0,
                    )?;

                    let x_sq = a.value().map(|x| *x * x);
                    let y_sq = b.value().map(|y| *y * y);
                    let val = x_sq + y_sq;
                    let out = region.assign_advice(|| "c", config.c, 0, || val)?;

                    config.selector.enable(&mut region, 0)?;

                    // out.copy_advice(|| "", region, column, offset)
                    Ok((b, out))
                },
            )?;

            for i in 2..10 {
                let c_new = layouter.namespace(|| format!("row_{i}")).assign_region(
                    || format!("region_{i}"),
                    |mut region| {
                        config.selector.enable(&mut region, 0)?;
                        b.copy_advice(|| "copy_advice_b", &mut region, config.a, 0)?;
                        c.copy_advice(|| "copy_advice_out", &mut region, config.b, 0)?;

                        let x_sq = b.value().map(|x| *x * x);
                        let y_sq = c.value().map(|y| *y * y);
                        let val = x_sq + y_sq;
                        let out = region.assign_advice(|| "c", config.c, 0, || val)?;

                        Ok(out)
                    },
                )?;
                b = c;
                c = c_new;
            }

            for i in 10..20 {
                let c_new = layouter.namespace(|| format!("row_{i}")).assign_region(
                    || format!("xor_region_{i}"),
                    |mut region| {
                        config.xor_table.selector.enable(&mut region, 0)?;
                        b.copy_advice(|| "copy_advice_b", &mut region, config.a, 0)?;
                        c.copy_advice(|| "copy_advice_out", &mut region, config.b, 0)?;

                        let x_sq = b.value().map(|x| *x * x);
                        let y_sq = c.value().map(|y| *y * y);

                        let val = x_sq ^ y_sq;
                        let out = region.assign_advice(|| "c", config.c, 0, || val)?;

                        Ok(out)
                    },
                )?;
                b = c;
                c = c_new;
            }
            layouter.constrain_instance(c.cell(), config.instance, 2)
        }
    }

    fn fib_squared() -> Fr {
        let (mut a, mut b) = (Fr::from(0), Fr::from(1));
        for i in 1..10 {
            let c = a.square() + b.square();
            a = b;
            b = c;
        }
        for i in 1..10 {
            let c = a.square() ^ b.square();
            a = b;
            c = b;
        }

        b
    }
    #[test]
    fn test_works() {
        let k = 16;
        let res = fib_squared();

        let circuit = FibSquredCircuit::<Fr, 10> {
            a: Value::known(Fr::from(0)),
            b: Value::known(Fr::from(1)),
        };
        MockProver::run(k, &circuit, vec![vec![Fr::from(0), Fr::from(1), res]])
            .unwrap()
            .assert_satisfied();
    }
}
