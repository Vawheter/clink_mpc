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

use std::{
    io::{BufReader, BufWriter},
    os::unix::net::UnixStream,
};

const MIMC_ROUNDS: usize = 322;

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

    // let mut cot_receiver = CotReceiver::init(&mut channel, m, bs, &crs, &mut rng);
    let mut cot_receiver = CotReceiver::init(&mut channel, bs, &crs, &mut rng);
    
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