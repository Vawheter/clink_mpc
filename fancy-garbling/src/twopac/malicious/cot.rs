use curve::bn_256::{Fq, Fr};
use math::{test_rng, 
    One,
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
    pvw_sender: PVW_Sender,
}
    
impl CotSender {

    pub fn init<C: AbstractChannel, RNG: CryptoRng + Rng>(
        channel: &mut C,
        m: usize,
        rng: &mut RNG,
    ) -> Result<Self, Error> {
        let pvw_sender = PVW_Sender::init(channel, m, rng).unwrap();
        Ok(Self{ m, pvw_sender })
    }

    fn gen_otmsg<RNG: CryptoRng + Rng>(
        &mut self,
        rng: &mut RNG,
    ) -> (Vec<Fr>, Vec<Fr>, Vec<Fr>, Vec<Fr>) {
        let k_prgs_0 = (0..self.m).map(|_| rng.gen()).collect::<Vec<Fr>>();
        let k_prgs_1 = (0..self.m).map(|_| rng.gen()).collect::<Vec<Fr>>();
        let rs_0 = (0..self.m).map(|_| rng.gen()).collect::<Vec<Fr>>();
        let rs_1 = (0..self.m).map(|_| rng.gen()).collect::<Vec<Fr>>();

        (k_prgs_0, k_prgs_1, rs_0, rs_1)
    }

    fn enc_labels(
        &mut self,        
        labels_0: &Vec<Fr>,
        labels_1: &Vec<Fr>,
        k_prgs_0: &Vec<Fr>,
        k_prgs_1: &Vec<Fr>,
        mimc_constants: &Vec<Fr>,
    ) -> (Vec<Fr>, Vec<Fr>) {
        
        let mut cnt = Fr::one();
        let one = Fr::one();

        let mut enclabels_0 = vec![];
        let mut enclabels_1 = vec![];

        for i in 0..self.m {
            let key = mimc5_hash(&k_prgs_0[i], &cnt, &mimc_constants);
            enclabels_0.push(labels_0[i].add(key));
            cnt.add_assign(one);
        }

        cnt = one;

        for i in 0..self.m {
            let key = mimc5_hash(&k_prgs_1[i], &cnt, &mimc_constants);
            enclabels_1.push(labels_1[i].add(key));
            cnt.add_assign(one);
        }
        
        (enclabels_0, enclabels_1)
    }

    pub fn send_ot<C: AbstractChannel, RNG: CryptoRng + Rng>(
        &mut self,
        channel: &mut C,
        msgs_0: &Vec<Fr>,
        msgs_1: &Vec<Fr>,
        crs: &CRS,
        rng: &mut RNG,
    ) -> Result<(), Error> {
        self.pvw_sender.send(channel, &msgs_0, &msgs_1, &crs, rng).unwrap();
        Ok(())
    }

    pub fn send_enclabels<C: AbstractChannel>(
        &mut self,
        channel: &mut C,
        enclabels_0: &Vec<Fr>,
        enclabels_1: &Vec<Fr>,
    ) -> Result<(), Error> {
        for i in 0..self.m {
            channel.write_fr(&enclabels_0[i])?;
            channel.write_fr(&enclabels_1[i])?;
        }
        channel.flush()?;
        Ok(())
    }

    pub fn receive_mackeys<C: AbstractChannel>(
        &mut self,
        channel: &mut C,
    ) -> Result<Vec<Fr>, Error> {
        let mut mac_keys = vec![];
        for _ in 0..self.m {
            let k = channel.read_fr().unwrap();
            mac_keys.push(k);
        }
        Ok(mac_keys)
    }

    fn cmpt_macs(
        &mut self,
        labels_0: &Vec<Fr>,
        labels_1: &Vec<Fr>,
        rs_0: &Vec<Fr>,
        rs_1: &Vec<Fr>,
        mac_keys: &Vec<Fr>,
    ) -> (Vec<Fr>, Vec<Fr>) {

        let mut labelmacs_0 = vec![];
        let mut labelmacs_1 = vec![];

        for i in 0..self.m {
            let tmp = mac_keys[1].mul(labels_0[i].add(rs_0[i]));
            let mac = tmp.add(mac_keys[0].mul(rs_0[i]));
            labelmacs_0.push(mac);
        }
        
        for i in 0..self.m {
            let tmp = mac_keys[1].mul(labels_1[i].add(rs_1[i]));
            let mac = tmp.add(mac_keys[0].mul(rs_1[i]));
            labelmacs_1.push(mac);
        }
        
        (labelmacs_0, labelmacs_1)  
    }


    pub fn send_macs<C: AbstractChannel>(
        &mut self,
        channel: &mut C,
        labelmacs_0: &Vec<Fr>,
        labelmacs_1: &Vec<Fr>,
    ) -> Result<(), Error> {
        for i in 0..self.m {
            channel.write_fr(&labelmacs_0[i])?;
            channel.write_fr(&labelmacs_1[i])?;
        }
        channel.flush()?;
        Ok(())
    }

}

pub struct CotReceiver {
    m: usize,
    bs: Vec<bool>,
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
        let pvw_receiver = PVW_Receiver::init(channel, &crs, m, &bs, rng).unwrap();
        Self{ m, bs, pvw_receiver }
    }

    pub fn receive_ot<C: AbstractChannel>(
        &mut self,
        channel: &mut C,
    ) -> Result<Vec<Fr>, Error> {
        self.pvw_receiver.receive(channel, &self.bs)
    }

    pub fn receive_enclabels<C: AbstractChannel>(
        &mut self,
        channel: &mut C,
    ) -> Result<(Vec<Fr>, Vec<Fr>), Error> {
        let mut enclabels_0 = vec![];
        let mut enclabels_1 = vec![];
        for _ in 0..self.m {
            let el0 = channel.read_fr().unwrap();
            let el1 = channel.read_fr().unwrap();
            enclabels_0.push(el0);
            enclabels_1.push(el1);
        }
        Ok((enclabels_0, enclabels_1))
    }

    fn dec_labels(
        &mut self,
        enclabels_0: &Vec<Fr>,
        enclabels_1: &Vec<Fr>,
        k_prgs_b: &Vec<Fr>,
        mimc_constants: &Vec<Fr>,
    ) -> Vec<Fr> {

        let mut cnt = Fr::one();
        let one = Fr::one();

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

    fn gen_mac_keys<RNG: CryptoRng + Rng>(
        &mut self,
        rng: &mut RNG,
    ) -> Vec<Fr> {
        (0..2).map(|_| rng.gen()).collect::<Vec<Fr>>()
    }

    fn send_mac_keys<C: AbstractChannel>(
        &mut self,
        channel: &mut C,
        mac_keys: &Vec<Fr>,
    ) -> Result<(), Error> {
        for i in 0..self.m {
            channel.write_fr(&mac_keys[i]).unwrap();
        }
        channel.flush()?;
        Ok(())
    }

    pub fn receive_macs<C: AbstractChannel>(
        &mut self,
        channel: &mut C,
    ) -> Result<(Vec<Fr>, Vec<Fr>), Error> {
        let mut labelmacs_0 = vec![];
        let mut labelmacs_1 = vec![];
        for _ in 0..self.m {
            let lm0 = channel.read_fr().unwrap();
            let lm1 = channel.read_fr().unwrap();
            labelmacs_0.push(lm0);
            labelmacs_1.push(lm1);
        }
        Ok((labelmacs_0, labelmacs_1))
    }


    fn check_macs(
        &mut self,
        labelmacs_0: &Vec<Fr>,
        labelmacs_1: &Vec<Fr>,
        declabels_b: &Vec<Fr>,
        mac_keys: &Vec<Fr>,
        rs_b: &Vec<Fr>,
    ) -> bool {
        for i in 0..self.m {
            let tmp = mac_keys[1].mul(declabels_b[i].add(rs_b[i]));
            let labelmac_ = tmp.add(mac_keys[0].mul(rs_b[i]));
            if self.bs[i] {
                if labelmacs_1[i] != labelmac_ { return false; }
            } else {
                if labelmacs_0[i] != labelmac_ { return false; }
            }
        }
        true
    }

}

