use curve::bn_256::Fq;
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

    pub fn send_ot<C: AbstractChannel, RNG: CryptoRng + Rng>(
        &mut self,
        channel: &mut C,
        msgs_0: &Vec<Fq>,
        msgs_1: &Vec<Fq>,
        crs: &CRS,
        rng: &mut RNG,
    ) -> Result<(), Error> {
        self.pvw_sender.send(channel, &msgs_0, &msgs_1, &crs, rng).unwrap();
        Ok(())
    }

    pub fn send_enclabels<C: AbstractChannel>(
        &mut self,
        channel: &mut C,
        enclabels_0: &Vec<Fq>,
        enclabels_1: &Vec<Fq>,
    ) -> Result<(), Error> {
        for i in 0..self.m {
            channel.write_fq(&enclabels_0[i])?;
            channel.write_fq(&enclabels_1[i])?;
        }
        channel.flush()?;
        Ok(())
    }

    pub fn receive_mackeys<C: AbstractChannel>(
        &mut self,
        channel: &mut C,
    ) -> Result<Vec<Fq>, Error> {
        let mut mac_keys = vec![];
        for _ in 0..self.m {
            let k = channel.read_fq().unwrap();
            mac_keys.push(k);
        }
        Ok(mac_keys)
    }

    fn cmpt_macs(
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


    pub fn send_macs<C: AbstractChannel>(
        &mut self,
        channel: &mut C,
        labelmacs_0: &Vec<Fq>,
        labelmacs_1: &Vec<Fq>,
    ) -> Result<(), Error> {
        for i in 0..self.m {
            channel.write_fq(&labelmacs_0[i])?;
            channel.write_fq(&labelmacs_1[i])?;
        }
        channel.flush()?;
        Ok(())
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

        let pvw_receiver = PVW_Receiver::init(channel, &crs, m, &bs, rng).unwrap();
        Self{ m, bs, mac_keys, pvw_receiver }
    }

    pub fn receive_ot<C: AbstractChannel>(
        &mut self,
        channel: &mut C,
    ) -> Result<Vec<Fq>, Error> {
        self.pvw_receiver.receive(channel, &self.bs)
    }

    pub fn receive_enclabels<C: AbstractChannel>(
        &mut self,
        channel: &mut C,
    ) -> Result<(Vec<Fq>, Vec<Fq>), Error> {
        let mut enclabels_0 = vec![];
        let mut enclabels_1 = vec![];
        for _ in 0..self.m {
            let el0 = channel.read_fq().unwrap();
            let el1 = channel.read_fq().unwrap();
            enclabels_0.push(el0);
            enclabels_1.push(el1);
        }
        Ok((enclabels_0, enclabels_1))
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

    fn gen_mac_keys<RNG: CryptoRng + Rng>(
        &mut self,
        rng: &mut RNG,
    ) -> Vec<Fq> {
        (0..self.m).map(|_| rng.gen()).collect::<Vec<Fq>>()
    }

    fn send_mac_keys<C: AbstractChannel>(
        &mut self,
        channel: &mut C,
        mac_keys: &Vec<Fq>,
    ) -> Result<(), Error> {
        for i in 0..self.m {
            channel.write_fq(&mac_keys[i]).unwrap();
        }
        channel.flush()?;
        Ok(())
    }

    pub fn receive_macs<C: AbstractChannel>(
        &mut self,
        channel: &mut C,
    ) -> Result<(Vec<Fq>, Vec<Fq>), Error> {
        let mut labelmacs_0 = vec![];
        let mut labelmacs_1 = vec![];
        for _ in 0..self.m {
            let lm0 = channel.read_fq().unwrap();
            let lm1 = channel.read_fq().unwrap();
            labelmacs_0.push(lm0);
            labelmacs_1.push(lm1);
        }
        Ok((labelmacs_0, labelmacs_1))
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
    let m = 16;
    // why m = 1 reports error?

    let mut rng = test_rng();

    let crs_ = PVW_SetupMessy(&mut rng);
    let crs = &crs_.clone();

    let (sender, receiver) = UnixStream::pair().unwrap();

    let labels_0 = (0..m).map(|_| rng.gen()).collect::<Vec<Fq>>();
    let labels_0_ = labels_0.clone();

    let labels_1 = (0..m).map(|_| rng.gen()).collect::<Vec<Fq>>();
    let labels_1_ = labels_1.clone();

    let mimc_constants = (0..MIMC_ROUNDS).map(|_| rng.gen()).collect::<Vec<Fq>>();
    let mimc_constants_ = mimc_constants.clone();


    // Sender's thread
    let handle = std::thread::spawn(move || {
        let mut rng_ = test_rng();

        let reader = BufReader::new(sender.try_clone().unwrap());
        let writer = BufWriter::new(sender);
        let mut channel = Channel::new(reader, writer);

        let mut cot_sender = CotSender::init(&mut channel, m, labels_0, labels_1, &mut rng_).unwrap();
        let (k_prgs_0, k_prgs_1, rs_0, rs_1) = cot_sender.gen_otmsg(&mut rng_);
        
        // OT for (ki_0, ri_0) and (ki_1, ri_1)
        cot_sender.send_ot(&mut channel, &k_prgs_0, &k_prgs_1, &crs_, &mut rng_).unwrap();
        cot_sender.send_ot(&mut channel, &rs_0, &rs_1, &crs_, &mut rng_).unwrap();
        
        // Encrypt lables
        let (enclabels_0, enclabels_1) = cot_sender.enc_labels(&k_prgs_0, &k_prgs_1, &mimc_constants);

        // Send encrypted labels
        cot_sender.send_enclabels(&mut channel, &enclabels_0, &enclabels_1).unwrap();

        // Receive mac keys
        let mac_keys = cot_sender.receive_mackeys(&mut channel).unwrap();

        // Compute macs
        let (labelmacs_0, labelmacs_1) = cot_sender.cmpt_macs(&rs_0, &rs_1, &mac_keys);

        // Send macs
        cot_sender.send_macs(&mut channel, &labelmacs_0, &labelmacs_1).unwrap();

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
    
    // OT for (ki_0, ri_0) and (ki_1, ri_1)
    let k_prgs_b  = cot_receiver.receive_ot(&mut channel).unwrap();
    let rs_b  = cot_receiver.receive_ot(&mut channel).unwrap();

    // Receive encrypted labels
    let (enclabels_0, enclabels_1) = cot_receiver.receive_enclabels(&mut channel).unwrap();
    
    // Decrypt labels
    let declabels_b = cot_receiver.dec_labels(&enclabels_0, &enclabels_1, &k_prgs_b, &mimc_constants_);

    for i in 0..m {
        assert_eq!(declabels_b[i], if bs_[i] { labels_1_[i] } else { labels_0_[i] });
    }
    println!("Decryption done");

    // Generate mac keys
    let mac_keys = cot_receiver.gen_mac_keys(&mut rng);

    // Send mac keys
    cot_receiver.send_mac_keys(&mut channel, &mac_keys).unwrap();

    let (labelmacs_0, labelmacs_1) = cot_receiver.receive_macs(&mut channel).unwrap();
    println!("Receiving macs done");

    // Check macs
    cot_receiver.check_macs(&labelmacs_0, &labelmacs_1, &declabels_b, &rs_b);
    println!("Checking macs done");

    handle.join().unwrap();
}

