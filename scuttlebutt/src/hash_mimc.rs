// For randomness
use rand::Rng;

use curve::bn_256::Fr;
use math::{test_rng, 
    Field, 
    bytes:: ToBytes,
    };

use crate::Block;
// use std::arch::x86_64::*;

const MIMC_ROUNDS: usize = 110;

pub fn mimc5_hash_block(xl_block: Block, xr_block: Block) -> Block {
    let rng = &mut test_rng();
    let constants: Vec<Fr> = (0..MIMC_ROUNDS).map(|_| rng.gen()).collect::<Vec<Fr>>();

    let xl_array: [u8; 16] = xl_block.into();
    let xr_array: [u8; 16] = xr_block.into();
    let xl: Fr = Fr::from_random_bytes(&xl_array).unwrap();
    let xr: Fr = Fr::from_random_bytes(&xr_array).unwrap();

    let image = mimc5_hash(&xl, &xr, &constants);
    let mut image_vec = vec![];
    image.write(&mut image_vec).unwrap();
    // TODO: length
    // let image_array: [u8; 16] = match &image_vec[..16].try_into() {
    //     Ok(arr) => *arr,
    //     Err(_) => panic!("Expected a Vec of length {} but it was {}", 16, image_vec.len()),
    // };
    // Block::try_from_slice(&image_vec[..16]).unwrap()
    let mut image_array = [0u8; 16]; 
    image_array.copy_from_slice(&image_vec[..16]);
    Block::from(image_array)
}

/// 5-power MiMC Hash, xL, xR := xR + (xL + Ci)^5, xL
pub fn mimc5_hash<F: Field>(xl: &F, xr: &F, constants: &[F]) -> F {
    let mut xli = *xl;
    let mut xri = *xr;
    for i in 0..MIMC_ROUNDS {
        let mut xl_ci = xli;
        xl_ci.add_assign(&constants[i]);
        let mut tmp = xl_ci;
        tmp.square_in_place();
        tmp.square_in_place();
        tmp.mul_assign(&xl_ci);
        tmp.add_assign(&xri);
        xri = xli;
        xli = tmp;
    }
    xli
}


#[test]
fn test_mimc5_hash_block() {
    let x_u128 = rand::random::<u128>();
    let x = Block::from(x_u128);
    let y_u128 = rand::random::<u128>();
    let y = Block::from(y_u128);

    let image = mimc5_hash_block(x, y);

    println!("{:?}", image);
}