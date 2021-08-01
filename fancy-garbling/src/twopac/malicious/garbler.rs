// use crate::{errors::TwopacError, Fancy, FancyInput, FancyReveal, Garbler as Gb, Wire};
// use ocelot::ot::Sender as OtSender;
// use rand::{CryptoRng, Rng, SeedableRng};
// use scuttlebutt::{AbstractChannel, Block, SemiHonest};

// use cot::{CotSender, CotReceiver}
// use curve::bn_256::Fq;
// use math::{test_rng, 
//     One,
//     };

// /// Semi-honest garbler.
// pub struct Garbler<C, RNG> {
//     garbler: Gb<C, RNG>,
//     channel: C,
//     cot: CotSender,
//     rng: RNG,
// }

// impl<C, COT, RNG> std::ops::Deref for Garbler<C, RNG, COT> {
//     type Target = Gb<C, RNG>;
//     fn deref(&self) -> &Self::Target {
//         &self.garbler
//     }
// }

// impl<C, COT, RNG> std::ops::DerefMut for Garbler<C, RNG, COT> {
//     fn deref_mut(&mut self) -> &mut Gb<C, RNG> {
//         &mut self.garbler
//     }
// }

// impl<C: AbstractChannel, RNG: CryptoRng + Rng + SeedableRng<Seed = Block> > Garbler<C, RNG> {
//     /// Make a new `Garbler`.
//     pub fn new(
//         mut channel: C, 
//         m: usize,
//         mut rng: RNG
//     ) -> Result<Self, TwopacError> {
//         let ot = CotSender::init(&mut channel, m, &mut rng).unwrap();

//         let garbler = Gb::new(channel.clone(), RNG::from_seed(rng.gen()));
//         Ok(Garbler {
//             garbler,
//             channel,
//             ot,
//             rng,
//         })
//     }

//     /// Get a reference to the internal channel.
//     pub fn get_channel(&mut self) -> &mut C {
//         &mut self.channel
//     }

//     fn _evaluator_input(&mut self, delta: &Wire, q: u16) -> (Wire, Vec<(Block, Block)>) {
//         let len = f32::from(q).log(2.0).ceil() as u16;
//         let mut wire = Wire::zero(q);
//         let inputs = (0..len)
//             .map(|i| {
//                 let zero = Wire::rand(&mut self.rng, q);
//                 let one = zero.plus(&delta);
//                 wire = wire.plus(&zero.cmul(1 << i));
//                 (zero.as_block(), one.as_block())
//             })
//             .collect::<Vec<(Block, Block)>>();
//         (wire, inputs)
//     }
// }

// impl<C: AbstractChannel, RNG: CryptoRng + Rng + SeedableRng<Seed = Block> > FancyInput for Garbler<C, RNG>
// {
//     type Item = Wire;
//     type Error = TwopacError;

//     fn encode(&mut self, val: u16, modulus: u16) -> Result<Wire, TwopacError> {
//         let (mine, theirs) = self.garbler.encode_wire(val, modulus);
//         self.garbler.send_wire(&theirs)?;
//         self.channel.flush()?;
//         Ok(mine)
//     }

//     fn encode_many(&mut self, vals: &[u16], moduli: &[u16]) -> Result<Vec<Wire>, TwopacError> {
//         let ws = vals
//             .iter()
//             .zip(moduli.iter())
//             .map(|(x, q)| {
//                 let (mine, theirs) = self.garbler.encode_wire(*x, *q);
//                 self.garbler.send_wire(&theirs)?;
//                 Ok(mine)
//             })
//             .collect();
//         self.channel.flush()?;
//         ws
//     }

//     fn receive_many(&mut self, qs: &[u16]) -> Result<Vec<Wire>, TwopacError> {
//         let n = qs.len();
//         let lens = qs.iter().map(|q| f32::from(*q).log(2.0).ceil() as usize);
//         let mut wires = Vec::with_capacity(n);
//         let mut inputs = Vec::with_capacity(lens.sum());

//         for q in qs.iter() {
//             let delta = self.garbler.delta(*q);
//             let (wire, input) = self._evaluator_input(&delta, *q);
//             wires.push(wire);
//             for i in input.into_iter() {
//                 inputs.push(i);
//             }
//         }
//         self.ot.send(&mut self.channel, &inputs, &mut self.rng)?;
//         Ok(wires)
//     }
// }

// impl<C: AbstractChannel, RNG: CryptoRng + Rng> Fancy for Garbler<C, RNG> {
//     type Item = Wire;
//     type Error = TwopacError;

//     fn constant(&mut self, x: u16, q: u16) -> Result<Self::Item, Self::Error> {
//         self.garbler.constant(x, q).map_err(Self::Error::from)
//     }

//     fn add(&mut self, x: &Wire, y: &Wire) -> Result<Self::Item, Self::Error> {
//         self.garbler.add(x, y).map_err(Self::Error::from)
//     }

//     fn sub(&mut self, x: &Wire, y: &Wire) -> Result<Self::Item, Self::Error> {
//         self.garbler.sub(x, y).map_err(Self::Error::from)
//     }

//     fn cmul(&mut self, x: &Wire, c: u16) -> Result<Self::Item, Self::Error> {
//         self.garbler.cmul(x, c).map_err(Self::Error::from)
//     }

//     fn mul(&mut self, x: &Wire, y: &Wire) -> Result<Self::Item, Self::Error> {
//         self.garbler.mul(x, y).map_err(Self::Error::from)
//     }

//     fn proj(&mut self, x: &Wire, q: u16, tt: Option<Vec<u16>>) -> Result<Self::Item, Self::Error> {
//         self.garbler.proj(x, q, tt).map_err(Self::Error::from)
//     }

//     fn output(&mut self, x: &Self::Item) -> Result<Option<u16>, Self::Error> {
//         self.garbler.output(x).map_err(Self::Error::from)
//     }
// }

// impl<C: AbstractChannel, RNG: CryptoRng + Rng> FancyReveal for Garbler<C, RNG> {
//     fn reveal(&mut self, x: &Self::Item) -> Result<u16, Self::Error> {
//         self.garbler.reveal(x).map_err(Self::Error::from)
//     }
// }

// // impl<C, RNG, COT> SemiHonest for Garbler<C, RNG, COT> {}
