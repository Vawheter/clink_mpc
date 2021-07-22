// For randomness (during paramgen and proof generation)
use rand::Rng;

// For benchmarking
use std::time::{Duration, Instant};

use math::One;

// Bring in some tools for using pairing-friendly curves
use curve::bn_256::{Bn_256, Fr};
use math::{test_rng, Field};

use scheme::clinkv2::r1cs::{ConstraintSynthesizer, ConstraintSystem, SynthesisError};

// We're going to use the BN-256 pairing-friendly elliptic curve.

// We'll use these interfaces to construct our circuit.

const MIMC_ROUNDS: usize = 110;
const SAMPLES: usize = 16000; //1048576//131070;//1048570;//131070;//16380;//16380;//16384

fn mimc<F: Field>(mut xl: F, mut xr: F, constants: &[F]) -> F {
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

struct MiMCDemo<'a, F: Field> {
    xl: Option<F>,
    xr: Option<F>,
    constants: &'a [F],
}

impl<'a, F: Field> ConstraintSynthesizer<F> for MiMCDemo<'a, F> {
    fn generate_constraints<CS: ConstraintSystem<F>>(
        self,
        cs: &mut CS,
        index: usize,
    ) -> Result<(), SynthesisError> {
        assert_eq!(self.constants.len(), MIMC_ROUNDS);

        cs.alloc_input(|| "", || Ok(F::one()), index)?;

        // Allocate the first component of the preimage.
        let mut xl_value = self.xl;
        let mut xl = cs.alloc(
            || "preimage xl",
            || xl_value.ok_or(SynthesisError::AssignmentMissing),
            index,
        )?;

        // Allocate the second component of the preimage.
        let mut xr_value = self.xr;
        let mut xr = cs.alloc(
            || "preimage xr",
            || xr_value.ok_or(SynthesisError::AssignmentMissing),
            index,
        )?;

        for i in 0..MIMC_ROUNDS {
            // xL, xR := xR + (xL + Ci)^5, xL
            let cs = &mut cs.ns(|| format!("round {}", i));

            // x2 = (xL + Ci)^2
            let x2_value = xl_value.map(|mut e| {
                e.add_assign(&self.constants[i]);
                e.square_in_place();
                e
            });
            let x2 = cs.alloc(
                || "x2",
                || x2_value.ok_or(SynthesisError::AssignmentMissing),
                index,
            )?;

            if index == 0 {
                cs.enforce(
                    || "x2 = (xL + Ci)^2",
                    |lc| lc + xl + (self.constants[i], CS::one()),
                    |lc| lc + xl + (self.constants[i], CS::one()),
                    |lc| lc + x2,
                );
            }

            // x4 = x2 * x2
            let x4_value = x2_value.map(|mut e| {
                e.square_in_place();
                e
            });
            let x4 = cs.alloc(
                || "x4",
                || x4_value.ok_or(SynthesisError::AssignmentMissing),
                index,
            )?;

            if index == 0 {
                cs.enforce(
                    || "x4 = (xL + Ci)^4",
                    |lc| lc + x2,
                    |lc| lc + x2,
                    |lc| lc + x4,
                );
            }

            // new_xL = xR + (xL + Ci)^5
            // new_xL = xR + tmp * (xL + Ci)
            // new_xL - xR = tmp * (xL + Ci)
            let new_xl_value = xl_value.map(|mut e| {
                e.add_assign(&self.constants[i]);
                e.mul_assign(&x4_value.unwrap());
                e.add_assign(&xr_value.unwrap());
                e
            });

            let new_xl = if i == (MIMC_ROUNDS - 1) {
                // This is the last round, xL is our image and so
                // we allocate a public input.
                cs.alloc_input(
                    || "image",
                    || new_xl_value.ok_or(SynthesisError::AssignmentMissing),
                    index,
                )?
            } else {
                cs.alloc(
                    || "new_xl",
                    || new_xl_value.ok_or(SynthesisError::AssignmentMissing),
                    index,
                )?
            };

            if index == 0 {
                cs.enforce(
                    || "new_xL = xR + (xL + Ci)^5",
                    |lc| lc + x4,
                    |lc| lc + xl + (self.constants[i], CS::one()),
                    |lc| lc + new_xl - xr,
                );
            }

            // xR = xL
            xr = xl;
            xr_value = xl_value;

            // xL = new_xL
            xl = new_xl;
            xl_value = new_xl_value;
        }

        Ok(())
    }
}

#[test]
fn mimc_clinkv2_kzg10() {
    use scheme::clinkv2::kzg10::{
        create_random_proof, verify_proof, ProveAssignment, VerifyAssignment, KZG10,
    };

    println!("Running mimc_clinkv2...");

    // Generate the MiMC round constants
    let mut rng = &mut test_rng();
    let constants = (0..MIMC_ROUNDS).map(|_| rng.gen()).collect::<Vec<_>>();

    let n: usize = SAMPLES;
    let degree: usize = n.next_power_of_two();

    let mut crs_time = Duration::new(0, 0);
    let mut prove_time = Duration::new(0, 0);
    let mut verify_time = Duration::new(0, 0);

    // Create parameters for our circuit
    let start = Instant::now();
    let kzg10_pp = KZG10::<Bn_256>::setup(degree, false, &mut rng).unwrap();
    let (kzg10_ck, kzg10_vk) = KZG10::<Bn_256>::trim(&kzg10_pp, degree).unwrap();
    crs_time += start.elapsed();

    // Prover
    let mut prover_pa = ProveAssignment::<Bn_256>::default();

    let mut io: Vec<Vec<Fr>> = vec![];
    let mut output: Vec<Fr> = vec![];

    for i in 0..n {
        // Generate a random preimage and compute the image
        let xl = rng.gen();
        let xr = rng.gen();
        let image = mimc(xl, xr, &constants);
        output.push(image);

        {
            // Create an instance of our circuit (with the witness)
            let c = MiMCDemo {
                xl: Some(xl),
                xr: Some(xr),
                constants: &constants,
            };
            c.generate_constraints(&mut prover_pa, i).unwrap();
        }
    }
    let one = vec![Fr::one(); n];
    io.push(one);
    io.push(output);

    let prove_start = Instant::now();
    let proof = create_random_proof(&prover_pa, &kzg10_ck, rng).unwrap();
    prove_time += prove_start.elapsed();

    // Verifier
    let mut verifier_pa = VerifyAssignment::<Bn_256>::default();

    // Create an instance of our circuit (with the witness)
    let verify_c = MiMCDemo {
        xl: None,
        xr: None,
        constants: &constants,
    };
    verify_c
        .generate_constraints(&mut verifier_pa, 0usize)
        .unwrap();

    let verify_start = Instant::now();
    assert!(verify_proof(&verifier_pa, &kzg10_vk, &proof, &io).unwrap());
    verify_time += verify_start.elapsed();

    // Compute time

    let prove_time =
        prove_time.subsec_nanos() as f64 / 1_000_000_000f64 + (prove_time.as_secs() as f64);
    let verify_time =
        verify_time.subsec_nanos() as f64 / 1_000_000_000f64 + (verify_time.as_secs() as f64);
    let crs_time = crs_time.subsec_nanos() as f64 / 1_000_000_000f64 + (crs_time.as_secs() as f64);

    println!("{:?}", crs_time);
    println!("{:?}", prove_time);
    println!("{:?}", verify_time);
}

#[test]
fn mimc_clinkv2_ipa() {
    use blake2::Blake2s;
    use scheme::clinkv2::ipa::{
        create_random_proof, verify_proof, InnerProductArgPC, ProveAssignment, VerifyAssignment,
    };

    let mut rng = &mut test_rng();
    // Generate the MiMC round constants
    let constants = (0..MIMC_ROUNDS).map(|_| rng.gen()).collect::<Vec<_>>();

    let n: usize = SAMPLES;

    println!("Running mimc_clinkv2...");

    // println!("Creating KZG10 parameters...");
    let degree: usize = n.next_power_of_two();
    let mut crs_time = Duration::new(0, 0);

    // Create parameters for our circuit
    let start = Instant::now();

    let ipa_pp = InnerProductArgPC::<Bn_256, Blake2s>::setup(degree, &mut rng).unwrap();
    let (ipa_ck, ipa_vk) = InnerProductArgPC::<Bn_256, Blake2s>::trim(&ipa_pp, degree).unwrap();

    crs_time += start.elapsed();

    println!("Start prove prepare...");
    // Prover
    let prove_start = Instant::now();

    let mut prover_pa = ProveAssignment::<Bn_256, Blake2s>::default();

    let mut io: Vec<Vec<Fr>> = vec![];
    let mut output: Vec<Fr> = vec![];

    for i in 0..n {
        // Generate a random preimage and compute the image
        let xl = rng.gen();
        let xr = rng.gen();
        let image = mimc(xl, xr, &constants);
        output.push(image);

        {
            // Create an instance of our circuit (with the witness)
            let c = MiMCDemo {
                xl: Some(xl),
                xr: Some(xr),
                constants: &constants,
            };
            c.generate_constraints(&mut prover_pa, i).unwrap();
        }
    }
    let one = vec![Fr::one(); n];
    io.push(one);
    io.push(output);

    println!("Create prove...");
    // Create a clinkv2 proof with our parameters.
    let proof = create_random_proof(&prover_pa, &ipa_ck, rng).unwrap();
    let prove_time = prove_start.elapsed();

    let proof_bytes = postcard::to_allocvec(&proof).unwrap();
    println!("Clinkv2-ipa mimc proof...ok, size: {}", proof_bytes.len());

    // Verifier
    println!("Start verify prepare...");
    let verify_start = Instant::now();

    let mut verifier_pa = VerifyAssignment::<Bn_256, Blake2s>::default();

    // Create an instance of our circuit (with the witness)
    let verify_c = MiMCDemo {
        xl: None,
        xr: None,
        constants: &constants,
    };
    verify_c
        .generate_constraints(&mut verifier_pa, 0usize)
        .unwrap();

    println!("Start verify...");

    // Check the proof
    assert!(verify_proof(&verifier_pa, &ipa_vk, &proof, &io).unwrap());

    let verify_time = verify_start.elapsed();

    // Compute time

    let proving_avg =
        prove_time.subsec_nanos() as f64 / 1_000_000_000f64 + (prove_time.as_secs() as f64);
    let verifying_avg =
        verify_time.subsec_nanos() as f64 / 1_000_000_000f64 + (verify_time.as_secs() as f64);
    let crs_time = crs_time.subsec_nanos() as f64 / 1_000_000_000f64 + (crs_time.as_secs() as f64);

    println!("Generating CRS time: {:?}", crs_time);
    println!("Proving time: {:?}", proving_avg);
    println!("Verifying time: {:?}", verifying_avg);
}
