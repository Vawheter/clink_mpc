use curve::bn_256::{Fq, Fr, G1Affine, G1Projective};
use math::{test_rng, 
    Field, 
    PrimeField,
    bytes:: ToBytes,
    One, Zero,
    };

use rand::{CryptoRng, Rng};

use ocelot::{
    Error,
    ot::pvw::{ CRS, SetupMessy as PVW_SetupMessy, Sender as PVW_Sender, Receiver as PVW_Receiver },
    };

use scuttlebutt::{
    AbstractChannel, Channel,
    hash_mimc::mimc5_hash,
    };

use core::ops::{Add, Sub, Mul, AddAssign};

const MIMC_ROUNDS: usize = 322;


pub struct CotSender {
    m: usize,
    labels_0: Vec<Fq>,
    labels_1: Vec<Fq>,
    pvw_sender: PVW_Sender,
}
    
impl CotSender {

    fn init<C: AbstractChannel, RNG: CryptoRng + Rng>(
        channel: &mut C,
        m: usize,
        labels_0: Vec<Fq>,
        labels_1: Vec<Fq>,
        rng: &mut RNG,
    ) -> Result<Self, Error> {
        let pvw_sender = PVW_Sender::init(channel, m, rng).unwrap();
        Ok(Self{ m, labels_0, labels_1, pvw_sender })
    }

    fn gen_otmsg<RNG: CryptoRng + Rng>(
        &mut self,
        rng: &mut RNG,
    ) -> (Vec<Fq>, Vec<Fq>, Vec<Fq>, Vec<Fq>) {
        let k_prgs_0 = (0..self.m).map(|_| rng.gen()).collect::<Vec<Fq>>();
        let k_prgs_1 = (0..self.m).map(|_| rng.gen()).collect::<Vec<Fq>>();
        let rs_0 = (0..self.m).map(|_| rng.gen()).collect::<Vec<Fq>>();
        let rs_1 = (0..self.m).map(|_| rng.gen()).collect::<Vec<Fq>>();

        (k_prgs_0, k_prgs_1, rs_0, rs_1)
    }

    fn enc_labels(
        &mut self,
        k_prgs_0: &Vec<Fq>,
        k_prgs_1: &Vec<Fq>,
        mimc_constants: &Vec<Fq>,
    ) -> (Vec<Fq>, Vec<Fq>) {
        
        let mut cnt = Fq::one();
        let one = Fq::one();

        let mut enclabels_0 = vec![];
        let mut enclabels_1 = vec![];

        for i in 0..self.m {
            let key = mimc5_hash(&k_prgs_0[i], &cnt, &mimc_constants);
            enclabels_0.push(self.labels_0[i].add(key));
            cnt.add_assign(one);
        }

        cnt = one;

        for i in 0..self.m {
            let key = mimc5_hash(&k_prgs_1[i], &cnt, &mimc_constants);
            enclabels_1.push(self.labels_1[i].add(key));
            cnt.add_assign(one);
        }
        
        (enclabels_0, enclabels_1)
    }

    pub fn send <C: AbstractChannel, RNG: CryptoRng + Rng>(
        &mut self,
        channel: &mut C,
        crs: &CRS,
        rng: &mut RNG,
    ) -> Result<(), Error> {
        self.pvw_sender.send(channel, &self.labels_0, &self.labels_1, &crs, rng).unwrap();
        Ok(())
    }


    fn gen_macs(
        &mut self,
        rs_0: &Vec<Fq>,
        rs_1: &Vec<Fq>,
        mac_keys: &Vec<Fq>,
    ) -> (Vec<Fq>, Vec<Fq>) {

        let mut labelmacs_0 = vec![];
        let mut labelmacs_1 = vec![];

        for i in 0..self.m {
            let tmp = mac_keys[1].mul(self.labels_0[i].add(rs_0[i]));
            let mac = tmp.add(mac_keys[0].mul(rs_0[i]));
            labelmacs_0.push(mac);
        }
        
        for i in 0..self.m {
            let tmp = mac_keys[1].mul(self.labels_1[i].add(rs_1[i]));
            let mac = tmp.add(mac_keys[0].mul(rs_1[i]));
            labelmacs_1.push(mac);
        }
        
        (labelmacs_0, labelmacs_1)  
    }
}

pub struct CotReceiver {
    m: usize,
    bs: Vec<bool>,
    mac_keys: Vec<Fq>,
    pvw_receiver: PVW_Receiver,
}

impl CotReceiver {

    fn init<C: AbstractChannel, RNG: CryptoRng + Rng>(
        channel: & mut C,
        m: usize,
        bs: Vec<bool>,
        crs: &CRS,
        rng: &mut RNG,
    ) -> Self {
        let mac_keys = (0..2).into_iter()
                             .map(|_| rng.gen())
                             .collect::<Vec<Fq>>();

        let mut pvw_receiver = PVW_Receiver::init(channel, &crs, m, &bs, rng).unwrap();
        Self{ m, bs, mac_keys, pvw_receiver }
    }

    fn dec_labels(
        &mut self,
        enclabels_0: &Vec<Fq>,
        enclabels_1: &Vec<Fq>,
        k_prgs_b: &Vec<Fq>,
        mimc_constants: &Vec<Fq>,
    ) -> Vec<Fq> {

        let mut cnt = Fq::one();
        let one = Fq::one();

        let mut declabels_b = vec![];
        
        for i in 0..self.m {
            let tmp = mimc5_hash(&k_prgs_b[i], &cnt, &mimc_constants);
            cnt.add_assign(one);
            if self.bs[i] { 
                declabels_b.push(enclabels_1[i].sub(tmp)); 
            } else { 
                declabels_b.push(enclabels_0[i].sub(tmp)); 
            } 

        }

        declabels_b
    }

    pub fn receive<C: AbstractChannel>(
        &mut self,
        channel: &mut C,
    ) -> Result<Vec<Fq>, Error> {
        self.pvw_receiver.receive(channel, &self.bs)
    }

    fn check_macs(
        &mut self,
        labelmacs_0: &Vec<Fq>,
        labelmacs_1: &Vec<Fq>,
        declabels_b: &Vec<Fq>,
        rs_b: &Vec<Fq>,
    ) -> bool {
        for i in 0..self.m {
            let tmp = self.mac_keys[1].mul(declabels_b[i].add(rs_b[i]));
            let labelmac_ = tmp.add(self.mac_keys[0].mul(rs_b[i]));
            if self.bs[i] {
                if labelmacs_1[i] != labelmac_ { return false; }
            } else {
                if labelmacs_0[i] != labelmac_ { return false; }
            }
        }
        true
    }

}

use std::{
    io::{BufReader, BufWriter},
    os::unix::net::UnixStream,
};

#[test]
fn test_cot()
{
    let m = 1;
    let mut rng = test_rng();

    let crs_ = PVW_SetupMessy(&mut rng);
    let crs = &crs_.clone();

    let (sender, receiver) = UnixStream::pair().unwrap();

    let labels_0 = (0..m).map(|_| rng.gen()).collect::<Vec<Fq>>();
    let labels_0_ = labels_0.clone();

    let labels_1 = (0..m).map(|_| rng.gen()).collect::<Vec<Fq>>();
    let labels_1_ = labels_1.clone();


    // Sender's thread
    let handle = std::thread::spawn(move || {
        let mut rng_ = test_rng();

        let reader = BufReader::new(sender.try_clone().unwrap());
        let writer = BufWriter::new(sender);
        let mut channel = Channel::new(reader, writer);

        let mut cot_sender = CotSender::init(&mut channel, m, labels_0, labels_1, &mut rng_).unwrap();
        // let (k_prgs_0, k_prgs_1, rs_0, rs_1) = cot_sender.gen_otmsg(&mut rng_);
        // for i in 0..m {
        //     cot_sender.pvw_sender.send(&mut channel, &labels_0[i], &labels_1[i], &crs_, &mut rng_).unwrap();
        // }
        cot_sender.send(&mut channel, &crs_, &mut rng_).unwrap();
    });

    // Receiver's thead
    let mut rng = test_rng();
    
    // Receiver's choosing bits
    let bs = (0..m).map(|_| rand::random::<bool>()).collect::<Vec<bool>>();
    let bs_ = bs.clone();

    let reader = BufReader::new(receiver.try_clone().unwrap());
    let writer = BufWriter::new(receiver);
    let mut channel = Channel::new(reader, writer);

    let mut cot_receiver = CotReceiver::init(&mut channel, m, bs, &crs, &mut rng);
    let result = cot_receiver.receive(&mut channel).unwrap();

    for i in 0..m {
        assert_eq!(result[i], if bs_[i] { labels_1_[i] } else { labels_0_[i] });
    }

    handle.join().unwrap();
}

