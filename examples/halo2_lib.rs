use clap::Parser;
use halo2_base::gates::{GateChip, GateInstructions};
use halo2_base::utils::ScalarField;
use halo2_base::AssignedValue;
use halo2_base::{gates::circuit::builder::BaseCircuitBuilder, QuantumCell};
#[allow(unused_imports)]
use halo2_base::{
    Context,
    QuantumCell::{Constant, Existing, Witness},
};
use halo2_scaffold::scaffold::cmd::Cli;
use halo2_scaffold::scaffold::run;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CircuitInput {
    pub x: [String; 3], // field element, but easier to deserialize as a string
    pub y: [String; 3],
}

// this algorithm takes a public input x, computes x^2 + 72, and outputs the result as public output
fn some_algorithm_in_zk<F: ScalarField>(
    builder: &mut BaseCircuitBuilder<F>,
    input: CircuitInput,
    make_public: &mut Vec<AssignedValue<F>>,
) {
    // let x = F::from_str_vartime(&input.x).expect("deserialize field element should not fail");
    // `Context` can roughly be thought of as a single-threaded execution trace of a program we want to ZK prove. We do some post-processing on `Context` to optimally divide the execution trace into multiple columns in a PLONKish arithmetization
    let ctx = builder.main(0);
    let x = input.x.map(|x| {
        ctx.load_witness(F::from_str_vartime(&x).expect("deserialize field element failed"))
    });
    let y = input.y.map(|y| {
        ctx.load_witness(F::from_str_vartime(&y).expect("deserialize field element y failed"))
    });
    // More advanced usage with multi-threaded witness generation is possible, but we do not explain it here

    // first we load a number `x` into as system, as a "witness"
    // let x = ctx.load_witness(x);
    // by default, all numbers in the system are private
    // we can make it public like so:
    for (a, b) in x.into_iter().zip(y) {
        make_public.push(a);
        make_public.push(b);
    }
    let mut res = F::ZERO;
    let mut witness_table: Vec<QuantumCell<F>> = Vec::default();
    let mut selector_table: Vec<isize> = Vec::default();
    for (i, (a, b)) in x.into_iter().zip(y).enumerate() {
        let product = *a.value() * b.value() + res;
        witness_table.push(Witness(res));
        witness_table.push(Existing(a));
        witness_table.push(Existing(b));
        witness_table.push(Witness(product));
        selector_table.push(4 * i as isize);
        res = product;
    }
    let _val_assigned = ctx.assign_region_last(witness_table, selector_table);
    // make_public.push(x);

    // create a Gate chip that contains methods for basic arithmetic operations
    // let gate = GateChip::<F>::default();

    // ===== way 1 =====
    // now we can perform arithmetic operations almost like a normal program using halo2-lib API functions
    // square x
    // let x_sq = gate.mul(ctx, x, x);
    //
    // // x^2 + 72
    // let c = F::from(72);
    // // the implicit type of most variables is an "Existing" assigned value
    // // a known constant is a separate type that we specify by `Constant(c)`:
    // let out = gate.add(ctx, x_sq, Constant(c));
    // // Halo2 does not distinguish between public inputs vs outputs because the verifier seems them all at the same time
    // // However in traditional terms, `out` is our output number. It is currently still private.
    // // Let's make it public:
    // make_public.push(out);
    // ==== way 2 =======
    // here is a more optimal way to compute x^2 + 72 using the lower level `assign_region` API:
    // let x_squared = *x.value() * x.value();
    // let val = x_squared * x.value() + c;
    // let _val_assigned = ctx.assign_region_last(
    //     [
    //         Constant(F::ZERO),
    //         Existing(x),
    //         Existing(x),
    //         Witness(x_squared),
    //         Constant(c),
    //         Witness(x_squared),
    //         Existing(x),
    //         Witness(val),
    //     ],
    //     [0, 4],
    // );
    // the `[0]` tells us to turn on a vertical `a + b * c = d` gate at row position 0.
    // this imposes the constraint c + x * x = val

    // ==== way 3 ======
    // this does the exact same thing as way 2, but with a pre-existing function
    // let _val_assigned = gate.mul_add(ctx, x, x, Constant(c));
    //
    // println!("x: {:?}", x.value());
    // println!("val_assigned: {:?}", out.value());
    // assert_eq!(*x.value() * x.value() + c, *out.value());
}

fn main() {
    env_logger::init();

    let args = Cli::parse();

    // run different zk commands based on the command line arguments
    run(some_algorithm_in_zk, args);
}
