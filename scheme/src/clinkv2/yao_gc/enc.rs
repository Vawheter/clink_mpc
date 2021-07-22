// For randomness
use rand::Rng;

use curve::bn_256::Fr;
use math::{test_rng, Field};

const MIMC_ROUNDS: usize = 110;

/// 5-power MiMC Hash, xL, xR := xR + (xL + Ci)^5, xL
fn mimc5<F: Field>(mut xl: F, mut xr: F, constants: &[F]) -> F {
    assert_eq!(constants.len(), MIMC_ROUNDS);

    for i in 0..MIMC_ROUNDS {
        let mut xl_ci = xl;
        xl_ci.add_assign(&constants[i]);
        let mut xx = xl_ci;
        xx.square_in_place();
        xx.square_in_place();
        xx.mul_assign(&xl_ci);
        xx.add_assign(&xr);
        xr = xl;
        xl = xx;
    }

    xl
}

#[test]
fn test_mimc5() {
    let rng = &mut test_rng();
    let constants: Vec<Fr> = (0..MIMC_ROUNDS).map(|_| rng.gen()).collect::<Vec<Fr>>();

    let xl = rng.gen();
    let xr = rng.gen();
    let image = mimc5(xl, xr, &constants);

    println!("{:?}", image);
}
