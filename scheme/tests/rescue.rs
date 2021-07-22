// The following code refers to Marvellous [https://github.com/KULeuven-COSIC/Marvellous] and Distaff [https://github.com/GuildOfWeavers/distaff]
// and thanks for their work
// @Author: JiadongLu (lujd1234@gmail.com)

use math::{test_rng, BitIterator, PrimeField};
use scheme::r1cs::{ConstraintSynthesizer, ConstraintSystem, SynthesisError};

// Hash Rescue utilizes Sponge Construction
// r, bitrate; c, capacity; M, state value, equal to r + c;
const _R: usize = 2;
const _C: usize = 1;
const M: usize = _R + _C;

const N: usize = 22; // round number

// ALPH * INVALPH == 1 (mod p-1)
// p == 0x30644e72e131a029b85045b68181585d2833e84879b9709143e1f593f0000001
const ALPH: [u64; 1] = [5];
const INVALPH: [u64; 4] = [
    0xcfe7f7a98ccccccd,
    0x535cb9d394945a0d,
    0x93736af8679aad17,
    0x26b6a528b427b354,
]; // 0x26b6a528b427b35493736af8679aad17535cb9d394945a0dcfe7f7a98ccccccd

/// This is an implementation of Resuce
/// See https://eprint.iacr.org/2019/426 for more
/// information about this construction.
///
/// ```
/// $Input:\ Plaintext\ P,\ round\ keys\ K_s\ for \ 0 ≤ s ≤ 2N$
/// $Output:\ Rescue\ (K, P)$
/// ​	$S_0 = P + K_0 $
/// ​	$for \ r = 1\ to\ N\ do$ // N round
/// ​		$for\ i = 1\ to\ m\ do$  // for every element
/// ​			$Interr[i] = K_{2r−1}[i] + ∑{m \atop j=1} M[i, j](S_{r−1}[j])^{1/α}$
/// ​		$end$
/// ​	$for\ i = 1\ to\ m\ do $
/// ​		$S_r[i] = K_{2r}[i] + ∑{m \atop j=1} M[i, j](Inter_r[j])^α $
/// ​	$end $
/// ​	$end$
/// ​$return\ S_N$
///

pub struct RescueConstant<F: PrimeField> {
    pub constants: [[F; M]; 2 * N + 1],
    pub mds: [[F; M]; M], // MDS matrix
}

fn str2p<F: PrimeField>(s: &str) -> F {
    let t = F::from_str(&String::from(s));
    let t = match t {
        Ok(t) => t,
        Err(_) => panic!("123"),
    };
    t
}

impl<F: PrimeField> RescueConstant<F> {
    pub fn new_fp255() -> RescueConstant<F> {
        let mut constants = [[F::zero(); M]; 2 * N + 1];
        let mut matrix = [[F::zero(); M]; M];
        for i in 0..2 * N + 1 {
            for j in 0..M {
                constants[i][j] = str2p(_CONSTANTS_CONSTANT[i][j]);
            }
        }
        for i in 0..M {
            for j in 0..M {
                matrix[i][j] = str2p(_CONSTANTS_MATRIX[i][j]);
            }
        }
        RescueConstant {
            constants: constants,
            mds: matrix,
        }
    }
}

pub fn block_cipher<F>(state: &mut [F], rc: &RescueConstant<F>)
where
    F: PrimeField,
{
    for i in 0..M {
        state[i].add_assign(rc.constants[0][i]);
    }

    let mut af: &[u64];
    for i in 0..2 * N {
        af = &ALPH;
        if i % 2 == 1 {
            af = &INVALPH;
        }
        for j in 0..M {
            state[j] = state[j].pow(af);
        }
        // matrix multiplication
        let mut tmp2 = [F::zero(); M];
        for j in 0..M {
            for k in 0..M {
                let mut t2 = rc.mds[j][k];
                t2.mul_assign(state[k]);
                tmp2[j] += t2;
            }
        }
        for j in 0..M {
            state[j] = tmp2[j] + rc.constants[i + 1][j];
        }
    }
}

pub fn rescue_hash<F: PrimeField>(xl: F, xr: F, constants: &RescueConstant<F>) -> F {
    let mut state = [xl, xr, F::zero()];
    block_cipher(&mut state, &constants);

    // c == 1
    state[0]
}

const _CONSTANTS_MATRIX: [[&str; 3]; 3] = [
    [
        "1727009077431585087915540656539954534780777332184950406202646746275199217608",
        "13540180854142779318387506093563803091263161877004989521740898372515032806705",
        "10612957846141125234745931852226986716584648319830558782678359506375923499159",
    ],
    [
        "5297766442418242490178207238435339074762251996759746754547126551178563896020",
        "8839191144842497789547275530090062910960637380659447844238808212289648631251",
        "11815940809319497569758915597574519690246728775737558100219686360032900828094",
    ],
    [
        "7380491535074041694005919481791705649661373282327647188984564535382271644294",
        "4073717453647711204997163819916439215770634741192958328872252346407126471025",
        "12611841555471625429590270932443304114397687320232742344089835243107459899095",
    ],
];

// (2 * N + 1) * M
const _CONSTANTS_CONSTANT: [[&str; 3]; 45] = [
    [
        "12769180023052250270311692729191801253904905578230519201957672068471212580693",
        "1521613489211025647495719579262333768015696501738145911352210890120622630165",
        "1026651289592356771951978355694508228824568426157920915274642188971734396101",
    ],
    [
        "12540922316541587696236134115889122422747332277293283596550877520791631080150",
        "1981000063383220069496173954274238217621235584192232023578205209215797100323",
        "5789795947997912880466289342703963180032067833105622704223174527643941061589",
    ],
    [
        "3569051189808788857184353212437216793616627759895253898513893939409784275151",
        "3700574960844093691174727002959422465392354647023911568223100145375939050927",
        "3389795102056716154944072963035322276363913344188885352913393242281651963951",
    ],
    [
        "3922179986724740239033875768756421762790876936762044072934302152539246316705",
        "9107062670761817214576003198540172938187448362948497717722951519317641272887",
        "11373086417573305160395459851359074326670257255008786832367972107171102482092",
    ],
    [
        "352020830305489436251321228146453931159813454684424638218069959229632878277",
        "14413234028989355754190955592750720458622821651527848940282906490720035993628",
        "7764588369571627500970668374971482221360194342851569455390056971510532800692",
    ],
    [
        "3019639905454726812405523304204733362154434449689241138984892664924662302907",
        "2032768739998524581122296664759089571735179263521009363114079188129174883916",
        "2543956343024053601883304672664559614650286398257967534316704482437319824487",
    ],
    [
        "1530541947142562750089219917333857094805444006643267052210048677389964447102",
        "2175671449313558868157095672252420325624392063860172265546466738232817104891",
        "369957040309597117443156424749309368651829224304198098436699933553602541498",
    ],
    [
        "13085992739863046435758396657923558184175585362598235458951521100313690188565",
        "4726695208567693044603223617921637684284878636653029166386612177675735569689",
        "450393225197503060679058635700250248836116980403818016541083261634693773640",
    ],
    [
        "4861802632256862298133271572945018464264126593419130453257627239516019552577",
        "2022199533602902092702625695403856201509532523674475454748527179165593355149",
        "10459073315014153850211155990153901716945821840788313471517150920283497179898",
    ],
    [
        "3925148963143604759472314334409450839580780476486833106362777517905787543413",
        "3464173052393235015114147419049561250640105143142295295214690847321831295736",
        "3469090550890205055808525336873278721019711577060277055069549607354239397009",
    ],
    [
        "4261860019004171720453120199086861695879599407649532562725279870742339002657",
        "14008098886741578492010855132216050237252291119032223886277210103577699148314",
        "435898237105648704400052065761827218489313344903667348846423723125142000451",
    ],
    [
        "11715073162248298180349869869650070707892118624222331746182132921654963183012",
        "8185062106151714338057645218120686728366622055884894938653175275099880596231",
        "13328013982961750894255372002298749693753232032147406388866437512154308887949",
    ],
    [
        "3923200053890284305942586540557407237552515492732457943419533456113723548763",
        "11442673351803285154717594151936223572404442703914017188604163955915716566111",
        "8976221480991702177436122904273488061244434621723921655693211049269573551995",
    ],
    [
        "12800451890367945160854310547256544304592111664194637750432015968022552198847",
        "9106293988894595212611998052807345304494748242684993446896803870239470379806",
        "2551305629934509588431154139989437800866619923603239398869146950746769410031",
    ],
    [
        "11295119646143840034576523696471492886685579876550673187676220693756912828512",
        "12412330473463282599741845067867733541569947911475863629371862880589644300993",
        "11619270384245046018901455869988958086181238257211698030289001218505660357441",
    ],
    [
        "11249633320963240508874222392397859215079644215917545871775027841004527010434",
        "10711889207558938636549788057912549242807150122479619843700306759735228413815",
        "7803596801157629275765041192365701131294588880185980648694409587507969568771",
    ],
    [
        "6221986694820499277091619136794532928343082379041937924485492681160885514974",
        "5295348018788379168039394424768656151521678381219932966800232102240315412104",
        "14364046246396109087880740778959731760537914670661158665938768246541909119221",
    ],
    [
        "13586987446115361097830003865910891150158985021411400316956855246132758121517",
        "9680160449310243126990906875921024419179289212537933159616587659805218753560",
        "8926096874114779357014109910737393263503490123253459436679451600160693271280",
    ],
    [
        "11845436939721374132665328559884949155986529706639009403153373809006057446237",
        "5970025112049920649636728401540451735417523716030215467430638479820547861202",
        "11556496857710081960508079301655461005773332489291670271998813658494207886552",
    ],
    [
        "1489007936365252964470009696711084964617230664998233380674740678250447036399",
        "11720049870415569473022954941485421798323722211828015514972094154492246825115",
        "4984800966858769390674202539041700531251718346204446329656708461081592932143",
    ],
    [
        "2374167092750843854076845245831407557165394747643889703163841106098061285779",
        "429760499963794534471130389712854792522159336065832162898818626514059602985",
        "1052957713396330082310672655931195653181859673306456599379265942595899287769",
    ],
    [
        "12262372623338121159380785299571462744499601875478522474240283101058411741807",
        "11083226227224775823196117587419199191769151725379903225264820189040513265745",
        "6500740773959440305622717487307199018888315892157391033270935126460555665232",
    ],
    [
        "2860353366121724200245879463381862902680404812486474834334907475266414671561",
        "13248030273823111623744756985649689301947946860555437724194299423000984021379",
        "12885848634578217346465633460868865632275250556971153897565223422521029847928",
    ],
    [
        "568475765045684786354555326168920526700021941987492866512362476576897980427",
        "6753357814198208689029239731026740115636658824113259212426541398070001197097",
        "13827484153414877909173304677817059978501932823131201177042524261895793907811",
    ],
    [
        "11624319772300326124706940308833493360055456191709578711818548866638742554474",
        "6008035343042878462187444796500380641372834810123728370479684263165430255068",
        "508985957289588153597455709572581205666205259580988202496754784385037350501",
    ],
    [
        "600578641192533696424538889015822304732421670513045450311924046470133343525",
        "5281710495362134173659242001984130849749225941742157456764286848097455350889",
        "3946664541918003520404233842179016841234698188825951784351059090165440776974",
    ],
    [
        "9027698350387752933019253957456801981954384037680698875250053199658854702479",
        "5518898806034299959145829575240729081723008854241188752246479585117426902268",
        "13793667333596113330459095925959774283719303563622403359515676750900631726904",
    ],
    [
        "14067325481846727624305849669986861764061986111465217211343204963379865686483",
        "11767737078735185499160405112418666301030464453613924515960631584781955420194",
        "5195006521836027645120475429816696847171695584574682858927226223480303485628",
    ],
    [
        "13331960063848339705906147327839479970756944295139311917213420517768143258614",
        "11386389255717082359969454464838821155750931784162672800806987842923152396421",
        "10470165792093946279055607245602412181463988702487554280963741471502266191923",
    ],
    [
        "11345266535595373101331596555155633660087591451017537987200504329270813219901",
        "7993664472107940638475626642276822343762234160016218445109225057906696518246",
        "3731316562144648099559260699380678576689015180634310023962453987156329343961",
    ],
    [
        "10969783265414589865903298647201032759986446662981873124474285674802366418595",
        "8292942772217643712878870675632883073436840305076909729559596452852440430049",
        "7039987094881050407486996972223621241096801960859041995492104631776680422212",
    ],
    [
        "5228687740295311096477591711667403867569907132097103370716053820815019829567",
        "12760105198964269539375035802061972675704015711579194355383469911258128042284",
        "5711410736886926182887703361043114221439493279258292077248617210528548723848",
    ],
    [
        "11005531869149636439792341152222642660844960489928597421833494173107037376117",
        "8963155598571985677441908841287779311235528074284106939331035898416968431064",
        "3214656040752122066374874120855740369246977034211286398843448687944097855949",
    ],
    [
        "2052858848446190101748680213827894166185705008068091757681246527899847213676",
        "7312940865962054351726210062898969719845224135729775192349766821005404298331",
        "10525772374942256225937290382549687506770132713862197210564731598923590837517",
    ],
    [
        "10278950458910814771717966491357541649240769760902406536782090198525733361107",
        "6501387756674884480154715990210166364479497358105334889034472293736242946774",
        "11532839058600455280337046279507907730469822816736536566619919502365453287155",
    ],
    [
        "12659267443780219988115658581156584236727416988944496176916079888050978224806",
        "11374202120081585046522336587996522954653929590304119501843255674789068074491",
        "13788946000945116056469902102693589007172711098697310258406434925640023751894",
    ],
    [
        "693202140241984421256137823081678421104203743181532766690867876770508249654",
        "11570246352215288360078790353796783802083901120186417426996878141463159172556",
        "1742549083003937131681818546255901914948971021415278411220903587736097675237",
    ],
    [
        "4171335681867263441001817935126480585799096875898897259348341324973784052835",
        "6435088212495642538750188443873824672038404647975316132522650301044936299634",
        "1886271982029080753740632359416734776971149947168622700632255709428022141359",
    ],
    [
        "13907909496169982010041254763855246431689828089031576982661914986201224651081",
        "365541470689095851150210957059972137736900865484253340161709304687880245095",
        "873413314290784831064319867993529025428327630622867664882030107100967825847",
    ],
    [
        "7567067495085267152648875599423100883901151545064406889027682727108018662355",
        "9639187277635629230351557441774366875481122932041719120594555808610905038183",
        "1298143984442957705461040734476343930164032030111867669585312181940515883624",
    ],
    [
        "1122328507210559149650049223968352413698467111320406102056392696849834955338",
        "7517645099815742221605197999606068976879566047916124677220029052889431206457",
        "1188316818774272152809100291450073280378298743754631627909345416994667517153",
    ],
    [
        "444005441375594077671633652217337557576437139572236044090962239037183185601",
        "12649535227544680176615410191610860707475581402297700742783236650948656039022",
        "2625532694658627018330710390959341896672074093357240486923959733043758570904",
    ],
    [
        "6943405427109186543695920735023593052123829193084941253048677323593432729230",
        "3227468143442519159359928834564713158322256819961857105808947411531048333686",
        "12102577061096560308065516125121975304286539491697852434561952018693242241632",
    ],
    [
        "7961821614851596194907602896335980392010536883641584393665360596091197566281",
        "11966595182453395873706530306621253962870321389774303442640369548066484794454",
        "442140402227407273169110282059652869846407911220907823127309527878220979688",
    ],
    [
        "1667517910232283327134702494336441516656419890135586118382191494532694994872",
        "436780978466842493250764099020001148205312245317766002571130039319917137898",
        "8452891184058066166352853279723599587385129727977812148359593483809594500167",
    ],
];

/// This is our demo circuit for proving knowledge of the
/// preimage of a Rescue hash invocation.

pub struct RescueDemo<'a, F: PrimeField> {
    pub xl: Option<F>,
    pub xr: Option<F>,
    pub constants: &'a RescueConstant<F>,
}

/// Our demo circuit implements this `Circuit` trait which
/// is used during paramgen and proving in order to
/// synthesize the constraint system.
impl<'a, F: PrimeField> ConstraintSynthesizer<F> for RescueDemo<'a, F> {
    fn generate_constraints<CS: ConstraintSystem<F>>(
        self,
        cs: &mut CS,
    ) -> Result<(), SynthesisError> {
        let xl_value = self.xl;
        let xl = cs.alloc(
            || "preimage xl",
            || xl_value.ok_or(SynthesisError::AssignmentMissing),
        )?;

        let xr_value = self.xr;
        let xr = cs.alloc(
            || "preimage xl",
            || xr_value.ok_or(SynthesisError::AssignmentMissing),
        )?;

        let three_value: Option<F> = Some(F::zero());
        let three = cs.alloc(
            || "preimage tmpf",
            || three_value.ok_or(SynthesisError::AssignmentMissing),
        )?;

        let mut state_value = [xl_value, xr_value, three_value];
        let mut state = [xl, xr, three];
        let cs = &mut cs.ns(|| format!("Preassign"));

        for i in 0..M {
            let tmp_value = state_value[i].map(|mut e| {
                e.add_assign(&self.constants.constants[0][i]);
                e
            });

            let tmp = cs.alloc(
                || "tmp",
                || tmp_value.ok_or(SynthesisError::AssignmentMissing),
            )?;

            cs.enforce(
                || "tmp = (state[i] + Ci) * 1",
                |lc| lc + state[i] + (self.constants.constants[0][i], CS::one()),
                |lc| lc + (F::one(), CS::one()),
                |lc| lc + tmp,
            );

            state_value[i] = tmp_value;
            state[i] = tmp;
        }

        let mut af: &[u64];
        for i in 0..2 * N {
            let cs = &mut cs.ns(|| format!("round {}", i));
            af = &ALPH;
            if i % 2 == 1 {
                af = &INVALPH;
            }

            for j in 0..M {
                let tuple = pow_with_constraint(&state_value[j], &state[j], af, cs)?;
                state_value[j] = tuple.0;
                state[j] = tuple.1;
            }

            let mut tmp2_value = [Some(F::zero()); M];
            let mut tmp2 = Vec::with_capacity(3);
            for j in 0..M {
                tmp2.push(cs.alloc(
                    || "tmp2[j]",
                    || tmp2_value[j].ok_or(SynthesisError::AssignmentMissing),
                )?);
            }

            for j in 0..M {
                for k in 0..M {
                    let tmp3_value: Option<F> = Some(self.constants.mds[j][k]);
                    let tmp3 = cs.alloc(
                        || "tmp3",
                        || tmp3_value.ok_or(SynthesisError::AssignmentMissing),
                    )?;

                    let new_tmp_value = tmp3_value.map(|mut e| {
                        e.mul_assign(&state_value[k].unwrap());
                        e.add_assign(&tmp2_value[j].unwrap());
                        e
                    });

                    let new_tmp = cs.alloc(
                        || "new tmp",
                        || new_tmp_value.ok_or(SynthesisError::AssignmentMissing),
                    )?;

                    cs.enforce(
                        || "new_tmp - tmp2[j] = tmp3_value * state_value[k]",
                        |lc| lc + tmp3,
                        |lc| lc + state[k],
                        |lc| lc + new_tmp - tmp2[j],
                    );

                    tmp2_value[j] = new_tmp_value;
                    tmp2[j] = new_tmp;
                }
            }
            for j in 0..M {
                let tmp_value = tmp2_value[j].map(|mut e| {
                    e.add_assign(&self.constants.constants[i + 1][j]);
                    e
                });

                let tmp = cs.alloc(
                    || "tmp",
                    || tmp_value.ok_or(SynthesisError::AssignmentMissing),
                )?;

                cs.enforce(
                    || "tmp = tmp2[j] + constants[i+1][j]",
                    |lc| lc + tmp2[j] + (self.constants.constants[i + 1][j], CS::one()),
                    |lc| lc + (F::one(), CS::one()),
                    |lc| lc + tmp,
                );

                state[j] = tmp;
                state_value[j] = tmp_value;
            }
        }

        let tmp = cs.alloc_input(
            || "input ",
            || state_value[0].ok_or(SynthesisError::AssignmentMissing),
        )?;

        cs.enforce(
            || "tmp = tmp2[j] + constants[i+1][j]",
            |lc| lc + (F::one(), CS::one()),
            |lc| lc + state[0],
            |lc| lc + tmp,
        );
        Ok(())
    }
}

fn pow_with_constraint<F: PrimeField, CS: ConstraintSystem<F>, S: AsRef<[u64]>>(
    state_value: &Option<F>,
    state: &scheme::r1cs::Variable,
    exp: S,
    cs: &mut CS,
) -> Result<(Option<F>, scheme::r1cs::Variable), SynthesisError> {
    let mut res_value: Option<F> = Some(F::one());
    let mut res = cs.alloc(
        || "res",
        || res_value.ok_or(SynthesisError::AssignmentMissing),
    )?;

    let mut found_one = false;
    for i in BitIterator::new(exp) {
        if !found_one {
            if i {
                found_one = true;
            } else {
                continue;
            }
        }

        let tmp_value = res_value.map(|mut e| {
            e.square_in_place();
            e
        });

        let tmp = cs.alloc(
            || "tmp",
            || tmp_value.ok_or(SynthesisError::AssignmentMissing),
        )?;

        cs.enforce(
            || "tmp = res * res",
            |lc| lc + res,
            |lc| lc + res,
            |lc| lc + tmp,
        );

        res_value = tmp_value;
        res = tmp;
        if i {
            let tmp_value = res_value.map(|mut e| {
                e.mul_assign(&(*state_value).unwrap());
                e
            });
            let tmp = cs.alloc(
                || "tmp",
                || tmp_value.ok_or(SynthesisError::AssignmentMissing),
            )?;

            cs.enforce(
                || "tmp = res * state",
                |lc| lc + res,
                |lc| lc + *state,
                |lc| lc + tmp,
            );
            res_value = tmp_value;
            res = tmp;
        }
    }

    Ok((res_value, res))
}

#[test]
fn test_rescue_groth16() {
    use rand::Rng;
    use scheme::groth16::{
        create_random_proof, generate_random_parameters, verifier::prepare_verifying_key,
        verify_proof,
    };
    use std::time::{Duration, Instant};

    let rng = &mut test_rng();
    use curve::bn_256::{Bn_256, Fr};
    let constants = RescueConstant::<Fr>::new_fp255();

    println!("Creating parameters...");

    let params = {
        let xl: Fr = rng.gen();
        let xr: Fr = rng.gen();

        let c = RescueDemo::<Fr> {
            xl: Some(xl),
            xr: Some(xr),
            constants: &constants,
        };

        generate_random_parameters::<Bn_256, _, _>(c, rng).unwrap()
    };

    let pvk = prepare_verifying_key(&params.vk);
    println!("Creating proofs...");

    // let's benchmark stuff!
    const SAMPLES: u32 = 3;
    let mut total_proving = Duration::new(0, 0);
    let mut total_verifying = Duration::new(0, 0);

    for _ in 0..SAMPLES {
        let xl: Fr = rng.gen();
        let xr: Fr = rng.gen();
        let image = rescue_hash(xl, xr, &constants);
        println!("xl {} xr {} \n hash: {}", xl, xr, image);
        {
            let start = Instant::now();
            let c = RescueDemo {
                xl: Some(xl),
                xr: Some(xr),
                constants: &constants,
            };

            let proof = create_random_proof(&params, c, rng).unwrap();
            total_proving += start.elapsed();
            let start = Instant::now();
            assert!(verify_proof(&pvk, &proof, &[image]).unwrap());
            total_verifying += start.elapsed();
        }
    }
    let proving_avg = total_proving / SAMPLES;
    let proving_avg =
        proving_avg.subsec_nanos() as f64 / 1_000_000_000f64 + (proving_avg.as_secs() as f64);

    let verifying_avg = total_verifying / SAMPLES;
    let verifying_avg =
        verifying_avg.subsec_nanos() as f64 / 1_000_000_000f64 + (verifying_avg.as_secs() as f64);

    println!("Average proving time: {:?} seconds", proving_avg);
    println!("Average verifying time: {:?} seconds", verifying_avg)
}

#[test]
fn test_rescue_spartan() {
    use curve::bn_256::{Bn_256, Fr};
    use rand::Rng;
    use scheme::spartan::prover::create_snark_proof;
    use scheme::spartan::r1cs::generate_r1cs;
    use scheme::spartan::setup::*;
    use scheme::spartan::spark::encode;
    use scheme::spartan::verify::verify_snark_proof;
    use std::time::{Duration, Instant};
    println!("\n spartan snark...");
    // This may not be cryptographically safe, use
    // `OsRng` (for example) in production software.
    let rng = &mut test_rng();

    let constants = RescueConstant::<Fr>::new_fp255();

    println!("Creating parameters...");
    let xl: Fr = rng.gen();
    let xr: Fr = rng.gen();

    let c = RescueDemo::<Fr> {
        xl: Some(xl),
        xr: Some(xr),
        constants: &constants,
    };

    println!("[snark_spartan]Generate parameters...");
    let r1cs = generate_r1cs::<Bn_256, _>(c).unwrap();

    let params = generate_setup_snark_parameters::<Bn_256, _>(
        rng,
        r1cs.num_aux,
        r1cs.num_inputs,
        r1cs.num_constraints,
    )
    .unwrap();
    println!("[snark_spartan]Generate parameters...ok");

    println!("[snark_spartan]Encode...");
    let (encode, encode_commit) = encode::<Bn_256, _>(&params, &r1cs, rng).unwrap();
    println!("[snark_spartan]Encode...ok");

    println!("Creating proofs...");

    // let's benchmark stuff!
    const SAMPLES: u32 = 3;
    let mut total_proving = Duration::new(0, 0);
    let mut total_verifying = Duration::new(0, 0);

    for _ in 0..SAMPLES {
        let xl: Fr = rng.gen();
        let xr: Fr = rng.gen();
        let image = rescue_hash(xl, xr, &constants);
        println!("xl {} xr {} \n hash: {}", xl, xr, image);
        {
            let start = Instant::now();
            let c = RescueDemo {
                xl: Some(xl),
                xr: Some(xr),
                constants: &constants,
            };

            let proof = create_snark_proof(&params, &r1cs, c, &encode, rng).unwrap();
            println!("[snark_spartan]Creating proof...ok");
            total_proving += start.elapsed();

            let start = Instant::now();
            println!("[snark_spartan]Verify proof...");
            let result = verify_snark_proof::<Bn_256>(
                &params,
                &r1cs,
                &vec![image].to_vec(),
                &proof,
                &encode_commit,
            )
            .is_ok();
            assert!(result);
            total_verifying += start.elapsed();
        }
    }
    let proving_avg = total_proving / SAMPLES;
    let proving_avg =
        proving_avg.subsec_nanos() as f64 / 1_000_000_000f64 + (proving_avg.as_secs() as f64);

    let verifying_avg = total_verifying / SAMPLES;
    let verifying_avg =
        verifying_avg.subsec_nanos() as f64 / 1_000_000_000f64 + (verifying_avg.as_secs() as f64);

    println!("Average proving time: {:?} seconds", proving_avg);
    println!("Average verifying time: {:?} seconds", verifying_avg)
}

#[test]
fn test_rescue_nizk_spartan() {
    use curve::bn_256::{Bn_256, Fr};
    use rand::Rng;
    use scheme::spartan::prover::create_nizk_proof;
    use scheme::spartan::r1cs::generate_r1cs;
    use scheme::spartan::setup::*;
    use scheme::spartan::verify::verify_nizk_proof;
    use std::time::{Duration, Instant};
    println!("\n spartan snark...");
    // This may not be cryptographically safe, use
    // `OsRng` (for example) in production software.
    let rng = &mut test_rng();

    let constants = RescueConstant::<Fr>::new_fp255();

    println!("Creating parameters...");
    let xl: Fr = rng.gen();
    let xr: Fr = rng.gen();

    let c = RescueDemo::<Fr> {
        xl: Some(xl),
        xr: Some(xr),
        constants: &constants,
    };

    let r1cs = generate_r1cs::<Bn_256, _>(c).unwrap();

    let params =
        generate_setup_nizk_parameters::<Bn_256, _>(rng, r1cs.num_aux, r1cs.num_inputs).unwrap();
    println!("[nizk_spartan]Generate parameters...ok");

    println!("Creating proofs...");

    // let's benchmark stuff!
    const SAMPLES: u32 = 3;
    let mut total_proving = Duration::new(0, 0);
    let mut total_verifying = Duration::new(0, 0);

    for _ in 0..SAMPLES {
        let xl: Fr = rng.gen();
        let xr: Fr = rng.gen();
        let image = rescue_hash(xl, xr, &constants);
        println!("xl {} xr {} \n hash: {}", xl, xr, image);
        {
            let start = Instant::now();
            let c = RescueDemo {
                xl: Some(xl),
                xr: Some(xr),
                constants: &constants,
            };

            let proof = create_nizk_proof(&params, &r1cs, c, rng).unwrap();
            println!("[nizk_spartan]Creating proof...ok");
            total_proving += start.elapsed();

            let start = Instant::now();
            println!("[nizk_spartan]Verify proof...");
            let result =
                verify_nizk_proof::<Bn_256>(&params, &r1cs, &vec![image].to_vec(), &proof).is_ok();
            assert!(result);
            total_verifying += start.elapsed();
        }
    }
    let proving_avg = total_proving / SAMPLES;
    let proving_avg =
        proving_avg.subsec_nanos() as f64 / 1_000_000_000f64 + (proving_avg.as_secs() as f64);

    let verifying_avg = total_verifying / SAMPLES;
    let verifying_avg =
        verifying_avg.subsec_nanos() as f64 / 1_000_000_000f64 + (verifying_avg.as_secs() as f64);

    println!("Average proving time: {:?} seconds", proving_avg);
    println!("Average verifying time: {:?} seconds", verifying_avg)
}
