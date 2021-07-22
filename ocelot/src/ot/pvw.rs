#![allow(non_snake_case)] 

use crate::errors::Error;

use curve::bn_256::{Fq, Fr, G1Affine, G1Projective};
use curve::{UniformRand, AffineCurve, ProjectiveCurve};
use math::{test_rng, 
    Field, 
    bytes:: ToBytes,
    Zero,
    };
use core::ops::{Add, Sub};

use rand::{CryptoRng, Rng};
use scuttlebutt::{AbstractChannel, Channel};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PublicKey {
    g: G1Affine,
    h: G1Affine,
}

type SecretKey = Fr;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CRS {
    g0: G1Affine,
    h0: G1Affine,
    g1: G1Affine,
    h1: G1Affine,
}

pub fn SetupMessy<RNG: CryptoRng + Rng>(
    rng: &mut RNG,
) -> CRS {
    let g0: G1Affine = G1Projective::rand(rng).into_affine();
    let g1: G1Affine = G1Projective::rand(rng).into_affine();
    let x0: Fr = rng.gen();
    let x1: Fr = rng.gen();
    assert_ne!(x0, x1);
    assert_ne!(x0, Fr::zero());
    assert_ne!(x1, Fr::zero());
    let h0 = g0.mul(x0).into_affine();
    let h1 = g1.mul(x1).into_affine();
    CRS {
        g0,
        h0,
        g1,
        h1,
    }
}


/// Oblivious transfer sender.
pub struct Sender {
    m: usize,
    pks: Vec<PublicKey>,
}

impl Sender {
    // type Msg = Block;

    pub fn init<C: AbstractChannel, RNG: CryptoRng + Rng>(
        channel: &mut C,
        m: usize,
        _: &mut RNG,
    ) -> Result<Self, Error> {
        let mut pks: Vec<PublicKey> = vec![];
        for _ in 0..m {
            let g = channel.read_pt()?;
            let h = channel.read_pt()?;
            pks.push(PublicKey{g, h});
        }
        Ok(Self { m, pks }) 
    }

    fn Randomize<RNG: CryptoRng + Rng>(
        g: &G1Affine,
        h: &G1Affine,
        g_star: &G1Affine,
        h_star: &G1Affine,
        rng: &mut RNG,
    ) -> (G1Affine, G1Affine) {
        let s: Fr = rng.gen();
        let t: Fr = rng.gen();
        let u = g.mul(s).add(h.mul(t)).into_affine();
        let v = g_star.mul(s).add(h_star.mul(t)).into_affine();
        (u, v)
    }

    
    fn DDHEnc<RNG: CryptoRng + Rng>(
        x: &Fq,
        g: &G1Affine,
        h: &G1Affine,
        g_star: &G1Affine,
        h_star: &G1Affine,
        rng: &mut RNG,
    ) -> (G1Affine, Fq) {
        let (u, v) = Sender::Randomize(g, h, g_star, h_star, rng);
        let vx = v.x.add(x);
        (u, vx)
    }

    fn Enc<RNG: CryptoRng + Rng>(
        &mut self,
        xs_0: &Vec<Fq>,
        xs_1: &Vec<Fq>,
        crs: &CRS,
        mut rng: &mut RNG,   
    ) -> (Vec<G1Affine>, Vec<Fq>, Vec<G1Affine>, Vec<Fq>) {
        let mut us_0:Vec<G1Affine> = vec![];
        let mut vxs_0:Vec<Fq> = vec![];
        let mut us_1:Vec<G1Affine> = vec![];
        let mut vxs_1:Vec<Fq> = vec![];
    
        for i in 0..self.m {
            let (u0, vx0) = Sender::DDHEnc(&xs_0[i], &crs.g0, &crs.h0, &self.pks[i].g, &self.pks[i].h, &mut rng);
            let (u1, vx1) = Sender::DDHEnc(&xs_0[i], &crs.g1, &crs.h1, &self.pks[i].g, &self.pks[i].h, &mut rng);
            us_0.push(u0);
            vxs_0.push(vx0);
            us_1.push(u1);
            vxs_1.push(vx1);
        }

        (us_0, vxs_0, us_1, vxs_1)
    }
    

    pub fn send<C: AbstractChannel, RNG: CryptoRng + Rng>(
        &mut self,
        channel: &mut C,
        xs_0: &Vec<Fq>,
        xs_1: &Vec<Fq>,
        crs: &CRS,
        rng: &mut RNG,
    ) -> Result<(), Error> {

        let (us_0, vxs_0, us_1, vxs_1) = self.Enc(&xs_0, &xs_1, &crs, rng);

        for i in 0..self.m {
            channel.write_pt(&us_0[i])?;

            let mut vx0_bytes = vec![];
            vxs_0[i].write(&mut vx0_bytes).unwrap();
            channel.write_bytes(&vx0_bytes)?;

            channel.write_pt(&us_1[i])?;

            let mut vx1_bytes = vec![];
            vxs_1[i].write(&mut vx1_bytes).unwrap();
            channel.write_bytes(&vx1_bytes)?;
        }

        channel.flush()?;
        Ok(())
    }
}

impl std::fmt::Display for Sender {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "PVW Sender")
    }
}


/// Oblivious transfer receiver.
pub struct Receiver {
    m: usize,
    sks: Vec<SecretKey>,
}

impl Receiver {
    // type Msg = Block;

    fn KeyGen<RNG: CryptoRng + Rng>(
        crs: &CRS,
        bs: &Vec<bool>,
        rng: &mut RNG,
    ) -> (Vec<PublicKey>, Vec<SecretKey>) {
    
        let mut pks: Vec<PublicKey> =  vec![];
        let mut sks: Vec<SecretKey> =  vec![];
    
        for b in bs.iter() {
            let r: Fr = rng.gen();
            assert_ne!(r, Fr::zero());
            sks.push(r);
    
            if *b {
                let g = crs.g1.mul(r).into_affine();
                let h = crs.h1.mul(r).into_affine();
                assert_ne!(g, G1Affine::zero());
                pks.push(PublicKey{g,h});
            } else {
                let g = crs.g0.mul(r).into_affine();
                let h = crs.h0.mul(r).into_affine();
                assert_ne!(g, G1Affine::zero());
                pks.push(PublicKey{g,h});
            }
        }
        (pks, sks)
    }
    

    pub fn init<C: AbstractChannel, RNG: CryptoRng + Rng>(
        channel: &mut C,
        crs: &CRS,
        m: usize,
        bs: &Vec<bool>,
        rng: &mut RNG,
    ) -> Result<Self, Error> {
        assert_eq!(m, bs.len());
        let (pks, sks) = Receiver::KeyGen(&crs, &bs, rng);
        for pk in pks.iter() {
            channel.write_pt(&pk.g)?;
            channel.write_pt(&pk.h)?;
        }
        channel.flush()?;
        Ok(Self{ m, sks })
    }


    fn Dec(
        sk: &SecretKey,
        c0: &G1Affine,
        c1: &Fq,
    ) -> Fq {
        c1.sub(c0.mul(*sk).into_affine().x)
    } 


    pub fn receive<C: AbstractChannel>(
        &mut self,
        channel: &mut C,
        bs: &Vec<bool>,
    ) -> Result<Vec<Fq>, Error> {

        let mut xs_b:Vec<Fq> = vec![];

        for i in 0..self.m {
            let u0 = channel.read_pt()?;

            let mut vx0_bytes = [0u8; 32];
            channel.read_bytes(&mut vx0_bytes).unwrap();
            let vx0 = Fq::from_random_bytes(&vx0_bytes).unwrap();

            let u1 = channel.read_pt()?;

            let mut vx1_bytes = [0u8; 32];
            channel.read_bytes(&mut vx1_bytes).unwrap();
            let vx1 = Fq::from_random_bytes(&vx1_bytes).unwrap();

            if bs[i] {
                let x1 = Receiver::Dec(&self.sks[i], &u1, &vx1);
                xs_b.push(x1);
            } else {
                let x0 = Receiver::Dec(&self.sks[i], &u0, &vx0);
                xs_b.push(x0);
            }
        }

        Ok(xs_b)
    }

}


impl std::fmt::Display for Receiver {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "PVW Receiver")
    }
}

// impl SemiHonest for Sender {}
// impl Malicious for Sender {}
// impl SemiHonest for Receiver {}
// impl Malicious for Receiver {}


use std::{
    io::{BufReader, BufWriter},
    os::unix::net::UnixStream,
};

// #[test]
// fn test_pvw_ot() 
// {
//     let mut rng = test_rng();
//     let m0: Fq = rng.gen();
//     let m1: Fq = rng.gen();

//     let crs = SetupMessy(& mut rng);
//     let crs_ = &crs.clone();

//     let (sender, receiver) = UnixStream::pair().unwrap();
//     let handle = std::thread::spawn(move || {
//         let mut rng = test_rng();
//         let reader = BufReader::new(sender.try_clone().unwrap());
//         let writer = BufWriter::new(sender);
//         let mut channel = Channel::new(reader, writer);
//         let mut pvw_sender = Sender::init(&mut channel, &mut rng).unwrap();
//         pvw_sender.send(&mut channel, &m0, &m1, &crs, &mut rng).unwrap();
//     });

//     let reader = BufReader::new(receiver.try_clone().unwrap());
//     let writer = BufWriter::new(receiver);
//     let mut channel = Channel::new(reader, writer);
//     let b: bool = rng.gen();
//     let mut pvw_receiver = Receiver::init(&mut channel, &crs_, &b, &mut rng).unwrap();
//     let result = pvw_receiver.receive(&mut channel, &b).unwrap();
//     assert_eq!(result, if b { m1 } else { m0 });

//     handle.join().unwrap();
// }
    
